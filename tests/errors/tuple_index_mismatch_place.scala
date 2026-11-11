val t = (1, 2)
def bad3: String = (1, 2)(1)
def bad4: String = t(1)
// scalac 3.8.4 places a mismatch on a tuple index's result where its call is, at the argument list's `(`, the
// receiver a value or a stable reference.
// expect: 2:26: error: Found:    Int
// expect: 3:21: error: Found:
