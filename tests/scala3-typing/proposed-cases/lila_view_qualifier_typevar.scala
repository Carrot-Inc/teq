// lila: `override def getNodeRenderingHandlers() = Set(NodeRenderingHandler(classOf[Image], ...)).asJava`
// where the Java method returns java.util.Set[NodeRenderingHandler[?]] and `asJava` comes from the
// implicit class scala.jdk.CollectionConverters.SetHasAsJava. Mimicked without the std here.
import scala.language.implicitConversions

class Handler[N](val cls: Class[N])     // invariant, like flexmark's Java NodeRenderingHandler<N>
class JSet[A](val elems: List[A])       // invariant, like java.util.Set<A>

implicit class SetHasAsJ[A](s: Set[A]):
  def asJ: JSet[A] = JSet(s.toList)

trait NodeRenderer:
  def handlers(): JSet[Handler[?]]

object ImageRenderer extends NodeRenderer:
  override def handlers() = Set(Handler(classOf[String])).asJ

@main def run() =
  println(ImageRenderer.handlers().elems.map(_.cls.getSimpleName))
