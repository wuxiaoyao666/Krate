import os


def encrypt_pdf(payload):
    import pikepdf
    """
    加密 PDF
    参数 payload 格式: { "input": "路径", "output": "路径", "password": "密码" }
    """
    input_path = payload.get("input")
    output_path = payload.get("output")
    password = payload.get("password")

    if not input_path or not os.path.exists(input_path):
        raise FileNotFoundError(f"找不到输入文件: {input_path}")

    # 使用 pikepdf 打开并加密
    with pikepdf.open(input_path) as pdf:
        enc = pikepdf.Encryption(
            user=password,
            owner=password,
            allow=pikepdf.Permissions(extract=False)  # 禁止复制文字
        )
        pdf.save(output_path, encryption=enc)

    return {
        "status": "success",
        "msg": "加密成功",
        "data": {"output_path": output_path}
    }


def decrypt_pdf(payload):
    import pikepdf
    """
    解密 PDF
    """
    input_path = payload.get("input")
    output_path = payload.get("output")
    password = payload.get("password")

    if not input_path or not os.path.exists(input_path):
        raise FileNotFoundError(f"找不到输入文件: {input_path}")

    try:
        # 尝试用密码打开
        with pikepdf.open(input_path, password=password) as pdf:
            # 直接保存即为解密
            pdf.save(output_path)

        return {
            "status": "success",
            "msg": "解密成功",
            "data": {"output_path": output_path}
        }
    except pikepdf.PasswordError:
        raise ValueError("密码错误，无法解密")
    except Exception as e:
        raise RuntimeError(f"解密发生未知错误: {str(e)}")