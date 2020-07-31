from bottle import route, run, template
@route('/')
def index():
    return "<h1>Scapegoat Server running on port 9000</h1><style>body{background-color: yellow;}</style>"

@route('/app/info')
def app_info():
    with open('./src/env.txt', 'r') as env:
        tmp = '<h1> Scapegoat App Info </h1>'
        for line in env.readlines():
            tmp = f'{tmp} <h5>{line}</h5>'
        return tmp

run(host='0.0.0.0', port=9000)
