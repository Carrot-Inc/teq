// A method whose parameter is a type member of a val's path is overridden in a subclass that
// overrides the val with a narrower type: the parameter seen from the subclass is read through the
// overriding val (dotty's `NamedType.withPrefix`, `infoDependsOnPrefix`), so the subclass's method
// is the override, and a call through the base dispatches to it (sttp's `AbstractFetchBackend`'s
// `handleStreamBody(s: streams.BinaryStream)` under `override val streams: ZioStreams`).
trait Streams { type Binary }
object IntStreams extends Streams { type Binary = Int }
abstract class Backend {
  val streams: Streams
  protected def handle(s: streams.Binary): String
  def send(s: streams.Binary): String = handle(s)
}
class IntBackend extends Backend {
  override val streams: IntStreams.type = IntStreams
  protected override def handle(s: Int): String = "handled " + (s + 1)
}
@main def run(): Unit =
  val b = IntBackend()
  println(b.send(41))
  val base: Backend { val streams: IntStreams.type } = b
  println(base.send(1))
