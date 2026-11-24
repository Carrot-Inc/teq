// The jar of tests/classpath/jvm/right_assoc_class_path_object_order.scala: two objects that run code when first
// loaded and one that runs nothing, each recording into `Log` when it does.
package objectinit

object Log:
  var out: List[String] = Nil

object LoudA:
  Log.out = Log.out :+ "a"
  override def toString = "LoudA"

object LoudB:
  Log.out = Log.out :+ "b"
  override def toString = "LoudB"

object Quiet:
  override def toString = "Quiet"
