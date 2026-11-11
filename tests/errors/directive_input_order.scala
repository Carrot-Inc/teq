// teq: tests/support/directive_order/B.scala tests/support/directive_order/A.scala
// The directives' rules in the order of the inputs as given (scala-cli's, not the program's by path): this file, B.scala
// (`any:s`), then A.scala (`any:e`), whose rule decides (docs/CLI.md).
object O { def f(): Unit = { 1; () } }
// expect: directive_input_order.scala:4:30: error: A pure expression does nothing in statement position
