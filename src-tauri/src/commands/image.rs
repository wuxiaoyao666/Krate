use image::GenericImageView;

#[tauri::command]
pub fn resize_image(input_path: String, output_path: String, width: u32, height: u32) -> Result<(), String> {
    // 打开图片
    let img = image::open(&input_path).map_err(|e| format!("打开图片失败: {}", e))?;

    // 执行调整大小
    // FilterType::Lanczos3 提供最好的质量
    let new_img = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);

    // 保存图片
    new_img.save(&output_path).map_err(|e| format!("保存失败: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn get_image_info(path: String) -> Result<(u32, u32), String> {
    let img = image::open(&path).map_err(|e| format!("读取失败: {}", e))?;
    Ok(img.dimensions())
}