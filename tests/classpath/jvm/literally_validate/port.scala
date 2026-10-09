// jars: scala-library literally
// std: scala-library
//> using dep org.typelevel::literally:1.2.0
// A `Literally` validator of the program in link mode: `validate(s)(using Quotes)` overrides
// the jar trait's member, so the output keeps it, and its quote is a stub that throws if ever
// called at run time; the literal itself expands at compile time.
import org.typelevel.literally.Literally
import scala.quoted.*

final case class Port(value: Int)
object Port:
  object PortLiteral extends Literally[Port]:
    def validate(s: String)(using Quotes): Either[String, Expr[Port]] =
      s.toIntOption.filter(p => p > 0 && p < 65536) match
        case Some(p) => Right('{ Port(${ Expr(p) }) })
        case None => Left(s"not a port: $s")

extension (inline ctx: StringContext)
  inline def port(inline args: Any*): Port = ${ Port.PortLiteral('ctx, 'args) }
