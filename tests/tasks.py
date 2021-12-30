from celery import Celery
import sys
import threading
import time

app = Celery('hello', broker='amqp://guest@localhost//', config_source={'broker_connection_retry': False, 'broker_transport_options': {'max_retries': 0}})

@app.task
def hello():
    return 'hello world'

class CeleryThread(threading.Thread):
    def run(self):
        app.start(sys.argv[1:])

if __name__ == '__main__':
    ct = CeleryThread()
    ct.setDaemon(True)
    ct.start()

    print(hello.delay())
    
    time.sleep(5)

    assert ct.is_alive()
    assert app.configured
