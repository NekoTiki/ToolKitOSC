//! The Direct3D 11 textures the panel's frames go to SteamVR through, one per overlay.
//!
//! `SetOverlayRaw` has SteamVR allocate a shared memory block for every frame, and they aren't all
//! given back: past 200 it refuses every upload ("Refusing to create memory block because 201
//! blocks are already outstanding" in its vrclient log) and the panel freezes on its last frame. A
//! texture is copied by SteamVR on our device instead, so nothing piles up.
use std::ffi::c_void;

use windows::core::Interface;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};

pub struct Textures {
    context: ID3D11DeviceContext,
    textures: [ID3D11Texture2D; 2],
    width: u32
}

impl Textures {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let error = |e: windows::core::Error| format!("Direct3D 11: {e}");
        let mut device = None;
        let mut context = None;
        unsafe {
            D3D11CreateDevice(None, D3D_DRIVER_TYPE_HARDWARE, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut context)).map_err(error)?;
        }
        let (device, context) = device.zip(context).ok_or("Direct3D 11: no device")?;
        let desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0
        };
        let texture = || -> Result<ID3D11Texture2D, String> {
            let mut texture = None;
            unsafe { device.CreateTexture2D(&desc, None, Some(&mut texture)).map_err(error)? };
            texture.ok_or_else(|| "Direct3D 11: no texture".into())
        };
        Ok(Self { textures: [texture()?, texture()?], context, width })
    }

    /// Copies a frame (RGBA, rows top to bottom) into texture `index` and hands it to `send`
    /// (`SetOverlayTexture`).
    pub fn upload<T>(&self, index: usize, rgba: &[u8], send: impl FnOnce(*mut c_void) -> T) -> T {
        let texture = &self.textures[index];
        unsafe { self.context.UpdateSubresource(texture, 0, None, rgba.as_ptr().cast(), self.width * 4, 0) };
        let sent = send(texture.as_raw());
        // SteamVR copies the texture into its own with a command on our context: until it's
        // flushed, the GPU doesn't run it and the overlay stays empty.
        unsafe { self.context.Flush() };
        sent
    }
}
