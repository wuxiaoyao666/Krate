import os
import platform
import shutil
import subprocess
from pathlib import Path


def _convert_with_docx2pdf(input_path: Path, output_path: Path):
    from docx2pdf import convert

    output_path.parent.mkdir(parents=True, exist_ok=True)
    convert(str(input_path), str(output_path))


def _convert_with_libreoffice(input_path: Path, output_path: Path):
    soffice = shutil.which("soffice")
    if not soffice:
        raise RuntimeError("当前系统未检测到 LibreOffice（soffice），无法完成转换")

    output_dir = output_path.parent
    output_dir.mkdir(parents=True, exist_ok=True)

    completed = subprocess.run(
        [
            soffice,
            "--headless",
            "--convert-to",
            "pdf",
            "--outdir",
            str(output_dir),
            str(input_path),
        ],
        capture_output=True,
        text=True,
        check=False,
    )

    if completed.returncode != 0:
        stderr = (completed.stderr or completed.stdout or "").strip()
        raise RuntimeError(f"LibreOffice 转换失败: {stderr}")

    generated_path = output_dir / f"{input_path.stem}.pdf"
    if not generated_path.exists():
        raise RuntimeError("LibreOffice 未生成目标 PDF 文件")

    if generated_path.resolve() != output_path.resolve():
        if output_path.exists():
            output_path.unlink()
        generated_path.replace(output_path)


def convert_word_to_pdf(payload):
    """
    Word 转 PDF
    payload: {"input": "路径", "output": "路径"}
    """
    input_path_raw = payload.get("input")
    output_path_raw = payload.get("output")

    if not input_path_raw:
        raise ValueError("缺少输入文件路径")
    if not output_path_raw:
        raise ValueError("缺少输出文件路径")

    input_path = Path(input_path_raw)
    output_path = Path(output_path_raw)

    if not input_path.exists():
        raise FileNotFoundError(f"找不到输入文件: {input_path}")

    if input_path.suffix.lower() not in {".doc", ".docx"}:
        raise ValueError("仅支持 .doc / .docx 文件")

    if output_path.suffix.lower() != ".pdf":
        output_path = output_path.with_suffix(".pdf")

    system = platform.system().lower()

    if system in {"windows", "darwin"}:
        _convert_with_docx2pdf(input_path, output_path)
    else:
        _convert_with_libreoffice(input_path, output_path)

    return {
        "status": "success",
        "msg": "转换成功",
        "data": {"output_path": str(output_path)},
    }
