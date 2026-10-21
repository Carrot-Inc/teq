/*
rules = DisableSyntax

DisableSyntax.noFinalize = true
*/
package test.disableSyntax

case object NoFinalize {

  class Simple {
    def finalize(): Unit = () // assert: DisableSyntax.noFinalize
  }

  class Protected {
    protected def finalize(): Unit = () // assert: DisableSyntax.noFinalize
  }

  class InferedUnit1 {
    protected def finalize() = () // assert: DisableSyntax.noFinalize
  }

  class InferedUnit {
    def finalize() = () // assert: DisableSyntax.noFinalize
  }

  class Example {
    protected def finalize() = () /* assert: DisableSyntax.noFinalize
                  ^^^^^^^^
    finalize should not be used
    */
  }

  class Negative {
    def ok = 1

    def finalize(in: Int): Int = in
  }
}