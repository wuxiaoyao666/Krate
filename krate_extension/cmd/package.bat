pip3 install -r requirements.txt
pyinstaller --noconsole --onefile --add-data "modules;modules" --collect-all pikepdf --name krate_extension main.py