// A macro's message at `Position.ofMacroExpansion`, or at no position, stands at the outermost
// inline call with the inline methods it was expanded through, as scalac's inline stack trace
// shows them ("This location contains code that was inlined from"), the expansion's position
// being the outermost call's.
// expect: main.scala:12:3: error: boom at the expansion
// expect: main.scala:13:3: error: boom at the expansion
// expect: inlined from tests/errors/macro_report_inlined/wrap.scala:2
// expect: main.scala:14:3: error: boom
// expect: inlined from tests/errors/macro_report_inlined/wrap.scala:3
// expect: 3 errors found
@main def run(): Unit =
  Pos.fail(true)
  Wrap.failWrapped
  Wrap.failPlain
