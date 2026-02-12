use base64::{engine::general_purpose, Engine as _};
use image::{ImageOutputFormat, RgbaImage};
use std::ffi::OsStr;
use std::io::Cursor;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

pub fn extract_icon_to_base64(path: &str) -> Result<String, String> {
    unsafe {
        // 1. 获取 HICON
        let wide_path: Vec<u16> = OsStr::new(path).encode_wide().chain(std::iter::once(0)).collect();
        let mut shfi = SHFILEINFOW::default();
        
        // SHGFI_ICON | SHGFI_LARGEICON 获取大图标 (通常是 32x32 或 48x48，取决于系统设置)
        let result = SHGetFileInfoW(
            PCWSTR(wide_path.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut shfi),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );

        if result == 0 {
            return Err(format!("Failed to extract icon from {}", path));
        }

        let hicon = shfi.hIcon;
        
        // 确保最终释放 HICON
        struct IconGuard(HICON);
        impl Drop for IconGuard {
            fn drop(&mut self) {
                unsafe { let _ = DestroyIcon(self.0); }
            }
        }
        let _icon_guard = IconGuard(hicon);

        // 2. 将 HICON 转换为 RgbaImage
        let image = icon_to_image(hicon).map_err(|e| e.to_string())?;

        // 3. 将 RgbaImage 转换为 PNG 并编码为 Base64
        let mut buffer = Cursor::new(Vec::new());
        image
            .write_to(&mut buffer, ImageOutputFormat::Png)
            .map_err(|e| format!("Failed to encode image to PNG: {}", e))?;

        let base64_string = general_purpose::STANDARD.encode(buffer.get_ref());
        Ok(base64_string)
    }
}

unsafe fn icon_to_image(hicon: HICON) -> Result<RgbaImage, String> {
    let mut icon_info = ICONINFO::default();
    if GetIconInfo(hicon, &mut icon_info).is_err() {
        return Err("GetIconInfo failed".to_string());
    }

    // 确保释放位图句柄
    struct BitmapGuard(HBITMAP);
    impl Drop for BitmapGuard {
        fn drop(&mut self) {
            unsafe { 
                if !self.0.is_invalid() { let _ = DeleteObject(HGDIOBJ(self.0.0)); }
            }
        }
    }
    let _color_guard = BitmapGuard(icon_info.hbmColor);
    let _mask_guard = BitmapGuard(icon_info.hbmMask);

    // 获取位图信息
    let mut bitmap = BITMAP::default();
    if GetObjectW(
        HGDIOBJ(icon_info.hbmColor.0), 
        size_of::<BITMAP>() as i32, 
        Some(&mut bitmap as *mut _ as *mut std::ffi::c_void)
    ) == 0 {
        return Err("GetObjectW failed".to_string());
    }

    let width = bitmap.bmWidth;
    let height = bitmap.bmHeight;
    let size = (width * height * 4) as usize;
    let mut pixels = vec![0u8; size];

    // 准备 BITMAPINFO
    let mut bi = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        biHeight: -height, // 负值表示从上到下的行顺序
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    
    let mut info = BITMAPINFO {
        bmiHeader: bi,
        ..Default::default()
    };

    // 获取设备上下文
    let dc = CreateCompatibleDC(HDC::default());
    struct DcGuard(HDC);
    impl Drop for DcGuard {
        fn drop(&mut self) {
            unsafe { let _ = DeleteDC(self.0); } 
        }
    }
    let _dc_guard = DcGuard(dc);

    // 获取像素数据
    if GetDIBits(
        dc,
        icon_info.hbmColor,
        0,
        height as u32,
        Some(pixels.as_mut_ptr() as *mut _),
        &mut info,
        DIB_RGB_COLORS,
    ) == 0 {
        return Err("GetDIBits failed".to_string());
    }

    // 处理像素格式：Windows 返回的是 BGRA，我们需要 RGBA
    // 同时需要处理 Alpha 通道（有时是预乘的，或者根本没有 Alpha）
    for chunk in pixels.chunks_mut(4) {
        let b = chunk[0];
        let g = chunk[1];
        let r = chunk[2];
        let a = chunk[3];
        
        chunk[0] = r;
        chunk[1] = g;
        chunk[2] = b;
        chunk[3] = a;
    }

    // 创建 Image 对象
    RgbaImage::from_raw(width as u32, height as u32, pixels)
        .ok_or_else(|| "Failed to create RgbaImage".to_string())
}
