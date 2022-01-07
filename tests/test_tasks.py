from celery import Celery
import sys
import threading
import time
import os

AMQP_HOST = os.environ.get("AMQP_HOST", "localhost")
print("host", AMQP_HOST)

app = Celery(
    "hello",
    broker=f"amqp://guest@{AMQP_HOST}//",
    backend="rpc://",
    config_source={
        "broker_connection_retry": False,
        "broker_transport_options": {"max_retries": 0},
    },
)


@app.task
def hello():
    return "hello world"


class CeleryThread(threading.Thread):
    def run(self):
        app.start(sys.argv[1:])


if __name__ == "__main__":
    ct = CeleryThread()
    ct.setDaemon(True)
    ct.start()

    res = hello.delay()
    print("hello", res)

    while True:
        print("res", res.ready())
        time.sleep(1)

    assert ct.is_alive()
    assert app.configured
