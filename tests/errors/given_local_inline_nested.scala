// A local `inline given` is an inline method (dotty's `Parsers.givenDef`), which an inline method's
// body may not define (`PrepareInlineable.checkInlineMethod`).
// expect: 6:12: error: Implementation restriction: nested inline methods are not supported
// expect: 1 error found
inline def twice(): (Int, Int) =
    inline given Int = 1
    (summon[Int], summon[Int])
@main def run(): Unit = println(twice())
