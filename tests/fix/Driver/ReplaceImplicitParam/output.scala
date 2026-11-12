// An implicit lambda's parameter spans its `implicit`: replaced with its name, the keyword gone with it
// (the output scalafix-core 0.14.9's).
package driver.replaceimplicitparam
object Subject {
  val block: Int => Int = { x => x }
  val expr: Int => Int = y => y
}
