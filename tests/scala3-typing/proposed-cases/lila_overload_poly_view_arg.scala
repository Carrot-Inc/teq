// flexmark's MutableDataSet has two generic overloads, set[T](DataKey[T], T) and
// set[T](NullableDataKey[T], T); lila calls `.set(Parser.HTML_BLOCK_PARSER, false)` where
// HTML_BLOCK_PARSER: DataKey[java.lang.Boolean], so `false` needs Predef.boolean2Boolean.
class DataKey[T](val name: String)
class NullableDataKey[T](val name: String)

class MutableDataSet:
  var entries: List[(String, Any)] = Nil
  def set[T](key: DataKey[T], value: T): MutableDataSet =
    entries = entries :+ (key.name -> value); this
  def set[T](key: NullableDataKey[T], value: T): MutableDataSet =
    entries = entries :+ (key.name -> value); this

val HTML_BLOCK_PARSER: DataKey[java.lang.Boolean] = DataKey("html-block")

@main def run() =
  val o = MutableDataSet().set(HTML_BLOCK_PARSER, false)
  println(o.entries)
