use std::marker::PhantomData;

use burn_tensor::backend::Backend;
use candle_core::DeviceLocation;

use crate::{
    element::{CandleElement, FloatCandleElement, IntCandleElement},
    CandleTensor,
};

/// Tensor backend that uses the [candle](candle_core) crate for executing tensor operations.
///
/// It is compatible with a wide range of hardware configurations, including CPUs and Nvidia GPUs
/// that support CUDA. Additionally, the backend can be compiled to `wasm` when using the CPU.
#[derive(Clone, Copy, Default, Debug)]
pub struct Candle<F = f32, I = i64>
where
    F: FloatCandleElement,
    I: IntCandleElement,
{
    _float: PhantomData<F>,
    _int: PhantomData<I>,
}

/// The device type for the candle backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// The device struct when using the `candle` backend.
///
/// Note that you need to provide the device index when using Cuda.
pub enum CandleDevice {
    /// CPU device.
    Cpu,

    /// Cuda device with the given index. The index is the index of the Cuda device in the list of
    /// all Cuda devices found on the system.
    Cuda(usize),
}

impl From<CandleDevice> for candle_core::Device {
    fn from(device: CandleDevice) -> Self {
        match device {
            CandleDevice::Cpu => candle_core::Device::Cpu,
            CandleDevice::Cuda(ordinal) => candle_core::Device::new_cuda(ordinal).unwrap(),
        }
    }
}

impl From<candle_core::Device> for CandleDevice {
    fn from(device: candle_core::Device) -> Self {
        match device.location() {
            DeviceLocation::Cpu => CandleDevice::Cpu,
            DeviceLocation::Cuda { gpu_id } => CandleDevice::Cuda(gpu_id),
            _ => panic!("Device unsupported: {device:?}"),
        }
    }
}

impl Default for CandleDevice {
    fn default() -> Self {
        Self::Cpu
    }
}

impl<F: FloatCandleElement, I: IntCandleElement> Backend for Candle<F, I> {
    type Device = CandleDevice;

    type FullPrecisionBackend = Candle<Self::FullPrecisionElem, Self::IntElem>;
    type FullPrecisionElem = f32;

    type TensorPrimitive<const D: usize> = CandleTensor<Self::FloatElem, D>;
    type FloatElem = F;

    type IntTensorPrimitive<const D: usize> = CandleTensor<Self::IntElem, D>;
    type IntElem = I;

    type BoolTensorPrimitive<const D: usize> = CandleTensor<u8, D>;

    fn ad_enabled() -> bool {
        false
    }

    fn name() -> String {
        "candle".to_string()
    }

    fn seed(seed: u64) {
        // TODO submit an issue at Candle
        panic!("Manual seed not supported by Candle. ")
    }
}
