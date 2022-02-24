def dec(*args):
    print(f"{args=}")

@dec
def f():
    print("f")

@dec("arg")
def g():
    print("g")

f()
g()
