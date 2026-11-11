package lss.lib

final class Holder(val pair: Option[(String, Int)])
final class Box(val value: Holder)

// Inline bodies whose match selector is no reference and has a singleton type all the same: a
// pattern definition over a stable path, which is pickled as `(p.x: p.x.type @unchecked) match`,
// and the ascription written out. The patterns are typed against the type under the singleton.
object Lib:
  inline def both(box: Box): String =
    val Some((k, n)) = box.value.pair: @unchecked
    val key: String = k
    key + n

  inline def size(box: Box): Int =
    (box.value.pair: box.value.pair.type) match
      case Some((k, n)) => k.length + n
      case None => 0
