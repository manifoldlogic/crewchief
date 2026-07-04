"""Python call-extraction accuracy fixture (spec F-D)."""


def validate(data):
    return bool(data)  # bool() is a builtin -> no edge


def transform(data):
    return data.upper()  # str.upper() has no chunk -> no edge


def process(data):
    if validate(data):          # process -> validate
        return transform(data)  # process -> transform
    return None


class Pipeline:
    def __init__(self, source):
        self.source = source

    def run(self):
        raw = self.load()       # run -> load (self method)
        return process(raw)     # run -> process (module function)

    def load(self):
        return self.source


def make_pipeline(src):
    # Pipeline is a class, not a callable kind -> instantiation makes NO edge.
    return Pipeline(src)
