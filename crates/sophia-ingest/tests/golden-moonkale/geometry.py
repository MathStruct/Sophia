class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y


def norm2(p):
    return p.x * p.x + p.y * p.y


def dist(a, b):
    d = Point(a.x - b.x, a.y - b.y)
    return norm2(d) ** 0.5


def is_even(n):
    if n == 0:
        return True
    return is_odd(n - 1)


def is_odd(n):
    if n == 0:
        return False
    return is_even(n - 1)
