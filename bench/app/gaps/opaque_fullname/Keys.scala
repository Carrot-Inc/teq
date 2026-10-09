package probe.opq
import scala.quoted.*

trait Key[K]:
  def keyName: String
  def wrap(raw: Long): K
  extension (k: K) def raw: Long

object Key:
  transparent inline def derived[K]: Key[K] = ${ KeyMacro.derive[K] }
  inline def apply[K](using k: Key[K]): k.type = k

trait LongKey[K]:
  inline given key: Key[K] = Key.derived[K]
  def apply(raw: Long): K = raw.asInstanceOf[K]

object KeyMacro:
  def derive[K: Type](using Quotes): Expr[Key[K]] =
    import quotes.reflect.*
    val tpe = TypeRepr.of[K]
    val sym = tpe.typeSymbol
    val name = Expr(sym.fullName)
    '{ new Key[K] {
         def keyName = $name
         def wrap(raw: Long): K = raw.asInstanceOf[K]
         extension (k: K) def raw: Long = k.asInstanceOf[Long]
       } }
