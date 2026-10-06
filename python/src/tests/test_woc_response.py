""" Test of the retries in the WoC get_response helper

Patches requests.get and time.sleep, so needs no network and does not wait.
"""
import unittest
from unittest.mock import MagicMock, patch

import requests

from tx_engine.interface import woc


def reply(status_code, body=None):
    response = MagicMock()
    response.status_code = status_code
    response.json.return_value = body
    return response


class WoCGetResponseTest(unittest.TestCase):
    """ Test of get_response retrying transient failures """

    def setUp(self):
        patcher = patch.object(woc.time, "sleep")
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_a_reset_connection_is_retried(self):
        # What failed CI: a reset raised from requests escaped the retry loop
        reset = requests.exceptions.ConnectionError(
            ("Connection aborted.", ConnectionResetError(104, "Connection reset by peer")))
        with patch.object(woc.requests, "get", side_effect=[reset, reply(200, {"ok": 1})]) as get:
            self.assertEqual(woc.get_response("https://example.invalid"), {"ok": 1})
        self.assertEqual(get.call_count, 2)

    def test_a_timeout_is_retried(self):
        with patch.object(woc.requests, "get",
                          side_effect=[requests.Timeout(), reply(200, [1])]):
            self.assertEqual(woc.get_response("https://example.invalid"), [1])

    def test_connection_errors_give_none_once_retries_run_out(self):
        with patch.object(woc.requests, "get",
                          side_effect=requests.ConnectionError()) as get:
            self.assertIsNone(woc.get_response("https://example.invalid", max_retries=3))
        self.assertEqual(get.call_count, 3)

    def test_a_transient_status_is_retried(self):
        with patch.object(woc.requests, "get",
                          side_effect=[reply(502), reply(429), reply(200, {"ok": 1})]):
            self.assertEqual(woc.get_response("https://example.invalid"), {"ok": 1})

    def test_a_permanent_status_is_not_retried(self):
        with patch.object(woc.requests, "get", side_effect=[reply(404)]) as get:
            self.assertIsNone(woc.get_response("https://example.invalid"))
        self.assertEqual(get.call_count, 1)


if __name__ == "__main__":
    unittest.main()
