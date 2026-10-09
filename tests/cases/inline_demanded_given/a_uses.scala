// The calls come first among the files, as in inline_demanded_this: `get`, `viaBlock` and
// `viaNested` expand at the call, the expansion asks for the result types of `value`, `number`
// and `nested`, which are inferred, and their bodies are typed on that demand. A search in such
// a body runs where the body stands: `summonInline` finds the given of `Holder`, not the one
// in scope at the call that asked, nor the one the asking method's block imports. A macro's
// run that calls `value` reads the same given.
package demandedgiven

given site: String = "site"

object Extra:
  given extra: Int = 42

inline def get: String = Holder.value

inline def viaBlock: Int =
  import Extra.given
  Holder.number

inline def viaNested: Int =
  import Extra.given
  Holder.nested

object Uses:
  def first(): String = get
  def second(): Int = viaBlock
  def third(): Int = viaNested
  def fourth(): Int = assertValue("own")

@main def run(): Unit =
  println(Uses.first())
  println(Uses.second())
  println(Uses.third())
  println(Uses.fourth())
  println(Holder.value)
  println(Holder.number)
  println(Holder.nested)
