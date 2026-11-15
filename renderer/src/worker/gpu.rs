use std::sync::Arc;

use wgpu::{Backend, DeviceType};

#[derive(clap::Args, Clone, Debug)]
pub struct GpuOptions {
    /// Index of the GPU adapter to use (see the adapter list logged at startup).
    /// If omitted, the most suitable adapter is selected automatically.
    #[clap(long, env)]
    pub gpu_adapter_index: Option<usize>,

    /// Allow software (CPU) rendering such as llvmpipe.
    /// Without this, the server refuses to start when no hardware GPU is available.
    #[clap(long, env, default_value_t = false)]
    pub allow_software_rendering: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum GpuError {
    #[error("GPUアダプタが1つも見つからない")]
    NoAdapter,

    #[error("GPUアダプタ #{index} は存在しない ({count} 個のアダプタが見つかった)")]
    AdapterIndexOutOfRange { index: usize, count: usize },

    #[error(
        "GPUアダプタ #{index} ({name}) はソフトウェアレンダラ (--allow-software-rendering で許可できる)"
    )]
    SoftwareAdapterNotAllowed { index: usize, name: String },

    #[error(
        "ハードウェアGPUアダプタが見つからない (ソフトウェアレンダリングは --allow-software-rendering で許可できる)"
    )]
    NoHardwareAdapter,

    #[error("デバイスの作成に失敗: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}

pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    pub async fn new(options: &GpuOptions) -> Result<Self, GpuError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());

        let adapters = instance.enumerate_adapters(wgpu::Backends::all()).await;

        for (i, adapter) in adapters.iter().enumerate() {
            let info = adapter.get_info();
            tracing::info!(
                "GPU Adapter #{i}: {} ({:?}, {:?}, {} {})",
                info.name,
                info.backend,
                info.device_type,
                info.driver,
                info.driver_info,
            );
        }

        let candidates: Vec<_> = adapters
            .iter()
            .map(|adapter| {
                let info = adapter.get_info();
                Candidate {
                    device_type: info.device_type,
                    backend: info.backend,
                    name: info.name,
                }
            })
            .collect();

        let index = select_adapter(&candidates, options)?;
        let adapter = &adapters[index];

        tracing::info!("GPU Adapter: #{index} ({})", adapter.get_info().name);

        // GLバックエンドでも動かせるよう、downlevel相当の制限に収める
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("eew-renderer"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                ..Default::default()
            })
            .await?;

        device.on_uncaptured_error(Arc::new(|error| {
            tracing::error!("UNRECOVERABLE ERROR (GPU): {error}");
            std::process::exit(1);
        }));

        device.set_device_lost_callback(|reason, message| {
            if matches!(reason, wgpu::DeviceLostReason::Destroyed) {
                return;
            }

            tracing::error!("UNRECOVERABLE ERROR (GPU device lost): {reason:?} {message}");
            std::process::exit(1);
        });

        Ok(Self { device, queue })
    }
}

struct Candidate {
    device_type: DeviceType,
    backend: Backend,
    name: String,
}

impl Candidate {
    fn is_software(&self) -> bool {
        self.device_type == DeviceType::Cpu
    }
}

fn select_adapter(candidates: &[Candidate], options: &GpuOptions) -> Result<usize, GpuError> {
    if candidates.is_empty() {
        return Err(GpuError::NoAdapter);
    }

    if let Some(index) = options.gpu_adapter_index {
        let candidate = candidates
            .get(index)
            .ok_or(GpuError::AdapterIndexOutOfRange {
                index,
                count: candidates.len(),
            })?;

        if candidate.is_software() && !options.allow_software_rendering {
            return Err(GpuError::SoftwareAdapterNotAllowed {
                index,
                name: candidate.name.clone(),
            });
        }

        return Ok(index);
    }

    candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| options.allow_software_rendering || !candidate.is_software())
        .min_by_key(|(_, candidate)| {
            (
                device_type_rank(candidate.device_type),
                backend_rank(candidate.backend),
            )
        })
        .map(|(index, _)| index)
        .ok_or(GpuError::NoHardwareAdapter)
}

fn device_type_rank(device_type: DeviceType) -> u8 {
    match device_type {
        DeviceType::DiscreteGpu => 0,
        DeviceType::IntegratedGpu => 1,
        DeviceType::VirtualGpu => 2,
        DeviceType::Other => 3,
        DeviceType::Cpu => 4,
    }
}

fn backend_rank(backend: Backend) -> u8 {
    match backend {
        Backend::Vulkan | Backend::Metal | Backend::Dx12 => 0,
        // NVIDIAのコンテナ環境ではGLバックエンドでデバイスを作れないことがあるので後回し
        Backend::Gl => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(device_type: DeviceType, backend: Backend) -> Candidate {
        Candidate {
            device_type,
            backend,
            name: format!("{device_type:?}/{backend:?}"),
        }
    }

    fn options(gpu_adapter_index: Option<usize>, allow_software_rendering: bool) -> GpuOptions {
        GpuOptions {
            gpu_adapter_index,
            allow_software_rendering,
        }
    }

    #[test]
    fn no_adapter() {
        assert!(matches!(
            select_adapter(&[], &options(None, true)),
            Err(GpuError::NoAdapter)
        ));
    }

    #[test]
    fn software_only_is_refused_by_default() {
        let candidates = [
            candidate(DeviceType::Cpu, Backend::Vulkan),
            candidate(DeviceType::Cpu, Backend::Gl),
        ];

        assert!(matches!(
            select_adapter(&candidates, &options(None, false)),
            Err(GpuError::NoHardwareAdapter)
        ));
    }

    #[test]
    fn software_only_is_selected_when_allowed() {
        let candidates = [
            candidate(DeviceType::Cpu, Backend::Gl),
            candidate(DeviceType::Cpu, Backend::Vulkan),
        ];

        assert_eq!(
            select_adapter(&candidates, &options(None, true)).unwrap(),
            1
        );
    }

    #[test]
    fn hardware_is_preferred_even_when_software_is_allowed() {
        let candidates = [
            candidate(DeviceType::Cpu, Backend::Vulkan),
            candidate(DeviceType::Other, Backend::Gl),
            candidate(DeviceType::IntegratedGpu, Backend::Vulkan),
            candidate(DeviceType::DiscreteGpu, Backend::Gl),
            candidate(DeviceType::DiscreteGpu, Backend::Vulkan),
        ];

        assert_eq!(
            select_adapter(&candidates, &options(None, true)).unwrap(),
            4
        );
    }

    #[test]
    fn explicit_software_adapter_requires_opt_in() {
        let candidates = [
            candidate(DeviceType::DiscreteGpu, Backend::Vulkan),
            candidate(DeviceType::Cpu, Backend::Vulkan),
        ];

        assert!(matches!(
            select_adapter(&candidates, &options(Some(1), false)),
            Err(GpuError::SoftwareAdapterNotAllowed { index: 1, .. })
        ));
        assert_eq!(
            select_adapter(&candidates, &options(Some(1), true)).unwrap(),
            1
        );
        assert!(matches!(
            select_adapter(&candidates, &options(Some(2), true)),
            Err(GpuError::AdapterIndexOutOfRange { index: 2, count: 2 })
        ));
    }
}
