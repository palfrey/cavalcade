https://www.rabbitmq.com/amqp-0-9-1-quickref.html

cargo install sqlx-cli --version ^0.5 --no-default-features --features postgres,rustls

docker-compose up -d db

cargo watch -s "./wait-for-it.sh --host=localhost --port=5672 --strict --timeout=0 -- bash -c 'cd tests && python test_tasks.py worker --loglevel=DEBUG'"

cargo sqlx prepare -- --lib