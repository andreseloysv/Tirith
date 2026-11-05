import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))
from shop import add, total


def test_add():
    assert add(2, 3) == 5


def test_total():
    assert total([1, 2, 3]) == 6
