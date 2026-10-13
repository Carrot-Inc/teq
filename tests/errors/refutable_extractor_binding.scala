// teq: --werror
// A pattern definition through a refutable extractor is warned of as dotty's `checkIrrefutable` warns
// (`RefutableExtractor`), over the pattern and the right side, a member's as a local's (a member's is
// desugared into a match with `MatchCheck.IrrefutablePatDef`); a generator's is the error with the
// `case` keyword's fix. Master gave a member none and a local the narrowing's message.
object E { def unapply(x: Int): Option[Int] = None }
object O { val E(x) = 1 }
def f(): Unit = { val E(y) = 1; () }
def g = for E(z) <- List(1) yield z
// expect: 7:16: warning: pattern binding uses refutable extractor `E`
// expect: If this usage is intentional, this can be communicated by adding `.runtimeChecked` after the expression,
// expect: 8:23: warning: pattern binding uses refutable extractor `E`
// expect: 9:13: error: pattern binding uses refutable extractor `E`
// expect: If this usage is intentional, this can be communicated by adding the `case` keyword before the full pattern,
