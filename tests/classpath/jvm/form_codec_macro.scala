// jars: scala-library tapir-core-jvm magnolia123-jvm sttp-model-core-jvm sttp-shared-core-jvm sttp-shared-ws-jvm
// std: scala-library
//> using dep com.softwaremill.sttp.tapir::tapir-core:1.13.29
// tapir's form codec for a case class: its macro makes each of the codec's two functions as a new method
// (`Symbol.newMethod` with a `MethodType`), its definition (`DefDef`, whose body is given the parameter's
// reference) and the closure over it, and the decoder's instance with `Select.unique(New(tpt), "<init>")`
// applied to `TypeApply(Select.unique(value, "asInstanceOf"), ..)` casts.
import sttp.tapir.*
import sttp.tapir.generic.Configuration

final case class Request(keyOne: String) derives Schema
object Request:
  given Codec[String, Request, CodecFormat.XWwwFormUrlencoded] =
    Codec.formCaseClassCodec(using Configuration.default.withSnakeCaseMemberNames)

@main def run(): Unit =
  val c = summon[Codec[String, Request, CodecFormat.XWwwFormUrlencoded]]
  println(c.encode(Request("abc")))
  println(c.decode("key_one=xyz"))
