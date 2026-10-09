// jars: scala-library abi-callbacks-lib
// std: scala-library
// Top-level vals, lazy vals and vars that the program never reads are kept in link mode and
// reached through the static forwarders of `f$package`.
import abi.Reflect
lazy val lz: Int = 3
val eager: String = "e"
var counter: Int = 5
@main def run(): Unit =
  println(Reflect.static("jvm_link_top_level_vals$package", "lz"))
  println(Reflect.static("jvm_link_top_level_vals$package", "eager"))
  println(Reflect.static("jvm_link_top_level_vals$package", "counter"))
