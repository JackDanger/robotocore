"""Tests for request metrics/counter."""

import threading

from robotocore.observability.metrics import RequestCounter


class TestRequestCounter:
    def test_initial_count_is_zero(self):
        counter = RequestCounter()
        assert counter.get("s3") == 0

    def test_increment_single_service(self):
        counter = RequestCounter()
        counter.increment("s3")
        counter.increment("s3")
        counter.increment("s3")
        assert counter.get("s3") == 3

    def test_increment_multiple_services(self):
        counter = RequestCounter()
        counter.increment("s3")
        counter.increment("sqs")
        counter.increment("s3")
        assert counter.get("s3") == 2
        assert counter.get("sqs") == 1

    def test_get_all(self):
        counter = RequestCounter()
        counter.increment("s3")
        counter.increment("sqs")
        counter.increment("s3")
        all_counts = counter.get_all()
        assert all_counts == {"s3": 2, "sqs": 1}

    def test_get_all_returns_copy(self):
        counter = RequestCounter()
        counter.increment("s3")
        all_counts = counter.get_all()
        all_counts["s3"] = 999
        assert counter.get("s3") == 1

    def test_reset(self):
        counter = RequestCounter()
        counter.increment("s3")
        counter.increment("sqs")
        counter.reset()
        assert counter.get("s3") == 0
        assert counter.get("sqs") == 0
        assert counter.get_all() == {}

    def test_uptime_seconds(self):
        counter = RequestCounter()
        assert counter.uptime_seconds >= 0

    def test_thread_safety(self):
        counter = RequestCounter()
        errors = []

        def increment_many():
            try:
                for _ in range(100):
                    counter.increment("s3")
            except Exception as e:
                errors.append(e)

        threads = [threading.Thread(target=increment_many) for _ in range(10)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        assert not errors
        assert counter.get("s3") == 1000

    def test_get_nonexistent_service(self):
        counter = RequestCounter()
        assert counter.get("nonexistent") == 0
