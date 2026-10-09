// jars: scala-library zio-jvm zio-stacktracer-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// std: scala-library
//> using dep dev.zio::zio:2.1.26
// A jar value class that only signatures name, with no selection on one of its values, so that
// the typer never completes it: zio's `ServiceWithZIOPartiallyApplied` over a `Boolean`. It
// erases as the jar's class says: every descriptor below takes and returns the `boolean` that
// the jar's `serviceWithZIO` returns, the field holds one, and the class loads under the verifier
// (a descriptor naming the class failed there with a bad return type). A box is made where `Any`
// takes the value and opened where a list hands one back, through the accessor the class file
// declares (`zio$ZIO$ServiceWithZIOPartiallyApplied$$dummy`).
import zio.*
import zio.ZIO.ServiceWithZIOPartiallyApplied

object Utils:
  def serviceM[T]: ServiceWithZIOPartiallyApplied[T] = ZIO.serviceWithZIO
  def pass[T](s: ServiceWithZIOPartiallyApplied[T]): ServiceWithZIOPartiallyApplied[T] = s
  def boxed: Any = serviceM[Int]
  def listed: List[ServiceWithZIOPartiallyApplied[Int]] = List(serviceM[Int], pass(serviceM[Int]))
  var field: ServiceWithZIOPartiallyApplied[String] = serviceM[String]
  def fromList: ServiceWithZIOPartiallyApplied[Int] = listed.head
  def loaded = "loaded"

object Main:
  def main(args: Array[String]): Unit =
    println(Utils.loaded)
    val local = Utils.serviceM[Int]
    val again = Utils.pass(local)
    println(Utils.boxed.getClass.getName)
    println(Utils.listed.size)
    val back = Utils.fromList
    println(List(back, again).size)
    Utils.field = Utils.serviceM[String]
