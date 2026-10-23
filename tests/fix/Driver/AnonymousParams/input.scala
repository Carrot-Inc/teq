/*
rules = ["file:../../support/ParamNames.scala"]
*/
// An anonymous parameter's name is scalameta's `Name.Anonymous`, its value empty, beside a named one's `Term.Name`.
package driver.anonymous
class C(using Int) /* assert: ParamNames.name
              ^^^
Name.Anonymous ``
*/
object O:
  def f(
      x: Int /* assert: ParamNames.name
      ^^^^^^
Term.Name `x`
*/
  ): Int = x
