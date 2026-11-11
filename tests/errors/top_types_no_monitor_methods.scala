// `Object`'s monitor methods are no member of `Any` (an `Int` reaches them through its box, `int2Integer`, a value of
// a top type not at all), and are an `AnyRef`'s, as scalac 3.8.4 reports them (L:C is where its caret begins,
// 1-based; scalac's message goes on with the extension-method hint of E008).
// expect: 6:25: error: value notify is not a member of Any
// expect: 1 error found
def f(x: Any): Unit = x.notify()
def h(x: AnyRef): Unit = x.notifyAll()
