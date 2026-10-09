package shop.codecs

trait Enc[T]:
  def enc(t: T): String

object Encs:
  private[shop] given intEnc: Enc[Int] with
    def enc(t: Int): String = s"i$t"

case class Sku(code: String)
object Sku:
  private[shop] given skuEnc: Enc[Sku] with
    def enc(s: Sku): String = s"sku:${s.code}"
