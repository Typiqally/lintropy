"""Tiny demo app to exercise lintropy's python rules."""


def greet(name: str) -> None:
    # TODO: switch to the logging module before shipping
    print(f"hi, {name}")


if __name__ == "__main__":
    greet("world")
    print("bye")
