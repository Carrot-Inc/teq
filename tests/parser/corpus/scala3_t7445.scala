// Adapted from scala3 tests/run/t7445.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
import scala.collection.immutable.ListMap

object Test {
	def main(args: Array[String]): Unit = ()
	val a = ListMap(1 -> 1, 2 -> 2, 3 -> 3, 4 -> 4, 5 -> 5);
	require(a.tail == ListMap(2 -> 2, 3 -> 3, 4 -> 4, 5 -> 5));
}
