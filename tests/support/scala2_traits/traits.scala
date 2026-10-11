// A Scala 2.13 library of traits alone (tests/support/jars.sh's scala2-traits): no module class, so only its class
// files' pickles say it is Scala 2's (tests/classpath/jvm/scala2_traits_only).
package scala2traits

trait Counter {
  private var n = 0
  def next: Int = { n += 1; n }
}

trait Named {
  def name: String
  def greet: String = "hello " + name
}

trait Flag {
  @volatile var on: Boolean = false
  var plain: Int = 0
}
