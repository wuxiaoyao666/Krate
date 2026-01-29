import sys
import json
import importlib
import os

# 1. 强制设置标准输出编码
sys.stdout.reconfigure(encoding='utf-8')
sys.stderr.reconfigure(encoding='utf-8')


def main():
    if getattr(sys, 'frozen', False):
        base_path = sys._MEIPASS
    else:
        base_path = os.path.dirname(os.path.abspath(__file__))

    if base_path not in sys.path:
        sys.path.append(base_path)

    if len(sys.argv) < 4:
        print(json.dumps({"status": "error", "msg": "参数不足"}, ensure_ascii=False))
        return

    module_name = sys.argv[1]  # e.g. pdf_ops
    func_name = sys.argv[2]  # e.g. encrypt_pdf
    json_str = sys.argv[3]  # e.g. {...}

    try:
        payload = json.loads(json_str)

        # 正常导入
        module = importlib.import_module(f"modules.{module_name}")

        if not hasattr(module, func_name):
            raise AttributeError(f"模块 {module_name} 中找不到方法 {func_name}")

        func = getattr(module, func_name)
        result = func(payload)

        print(json.dumps(result, ensure_ascii=False))

    except Exception as e:
        # 打印详细错误方便调试
        error_res = {
            "status": "error",
            "msg": f"{str(e)}",
            "debug_info": f"sys.path: {sys.path}"
        }
        print(json.dumps(error_res, ensure_ascii=False))


if __name__ == "__main__":
    main()