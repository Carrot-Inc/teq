package un

// 𝄞𝄢 two supplementary characters, two UTF-16 units and four bytes each, and é before the definitions
final class Notes(val clef: String):
  /** 𝅘𝅥𝅮 */
  def é: String = clef + "é"
  val 𝔵: Int = 1

object Notes:
  def treble: Notes = Notes("𝄞")
