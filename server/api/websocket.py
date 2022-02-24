from api import sio

@sio.event
def authenticate(sid, data):
    print(sid, data)
