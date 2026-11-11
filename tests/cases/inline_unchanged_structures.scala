// A copied inline body can share unchanged calls and lists. Each occurrence
// still evaluates its effects, and a copied binding belongs to its expansion.
object InlineEffects:
  var count = 0
  def next(): Int =
    count += 1
    count

  inline def pair(): Int = next() + next()
  inline def bound(): Int =
    val value = next()
    if value > 0 then value else 0
  inline def duplicate(inline value: Int): Int = value + value

@main def run(): Unit =
  println(InlineEffects.pair())
  println(InlineEffects.pair())
  println(InlineEffects.bound())
  println(InlineEffects.bound())
  println(InlineEffects.duplicate(InlineEffects.next()))
  println(InlineEffects.count)
