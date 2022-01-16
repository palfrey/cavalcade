Cavalcade
=========
[![Continuous integration](https://github.com/palfrey/cavalcade/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/palfrey/cavalcade/actions/workflows/ci.yml) 
![Docker Pulls](https://img.shields.io/docker/pulls/palfrey/cavalcade)
![Docker Image Version (latest semver)](https://img.shields.io/docker/v/palfrey/cavalcade)

Cavalcade is an [AMQP broker](https://en.wikipedia.org/wiki/Advanced_Message_Queuing_Protocol) using a PostgreSQL-compatible database as a backing store. It can work with PostgreSQL itself, but is primarily targeted at distributed PostgreSQL-compatible databases (e.g. [CockroachDB](https://www.cockroachlabs.com/), [YugabyteDB](https://www.yugabyte.com/yugabytedb/)) to enable a multi-node AMQP setup.

It's primary target of compatibility is "good enough to use as a drop-in replacement for [RabbitMQ](https://www.rabbitmq.com/) with [Celery](https://docs.celeryproject.org/)" but patches to improve that are welcomed.

Status
------
Experimental. If you want stability, go use [RabbitMQ](https://www.rabbitmq.com/) instead and use it's clustering support for multi-node. If you want to avoid dealing with the clustering, but are happy to trade debugging Cavalcade, welcome!

It has not been optimised at all for performance, and in some cases has actively sub-optimal choices in some of the database interactions (chosen because they worked v.s. taking a lot longer to built something optimal).

In all these cases, patches to improve things are welcomed.

Usage
-----

In all cases set `DATABASE_URL` to the URL for your database. Examples:
- PostgreSQL: `postgres://postgres:password@db/cavalcade`
- CockroachDB: `postgresql://root@crdb:26257/defaultdb?sslmode=disable`

Docker config:

  1. Pick an image from https://quay.io/repository/palfrey/cavalcade?tab=tags
  2. (Optionally): Download the [standard logging config](https://github.com/palfrey/cavalcade/blob/main/log4rs.yml), customise as required and mount in the image as `/log4rs.yml`
  3. Run the migration helper, then the main app. e.g `sh -c "/cavalcade --migrate --sqlx-path=/sqlx && /cavalcade"`. The former should do nothing on nodes without changes, and the two steps can be split if wished.

Local config:

  1. Download a release build from https://github.com/palfrey/cavalcade/releases (or checkout the repo and `cargo build` it yourself)
  2. [Install SQLx CLI](https://github.com/launchbadge/sqlx/tree/master/sqlx-cli)
  3. Copy the [migrations](https://github.com/palfrey/cavalcade/tree/main/migrations) for the tagged release you're using.
  4. Run `sqlx migrate run` in a folder with the migrations from step 3.
  5. (Optionally): Download the [standard logging config](https://github.com/palfrey/cavalcade/blob/main/log4rs.yml) and customise as required
  6. Run the release build without args - it assumes the logging config is in the same folder as it and that `DATABASE_URL` has been set.