package fix.overloads

// A trait whose parameterless member is overloaded, implemented by a case class field that a
// second trait declares alone (tapir's `EndpointInfoOps.info`, `EndpointMetaOps.info` and
// `Endpoint.info`), and a val beside an extension-like trait's method
// of its name with parameters (sttp's `PartialRequestExtensions.body(file)`).
trait InfoOps[A]:
  def info: String
  def withInfo(i: String): A
  def info(i: String): A = withInfo(i)
  def tag(t: String): A = withInfo(info + "," + t)

trait MetaOps:
  def info: String
  def show: String = "info " + info

final case class Ep(info: String, n: Int) extends InfoOps[Ep] with MetaOps:
  def withInfo(i: String): Ep = copy(info = i)

object Eps:
  def start: Ep = Ep("a", 1)

trait OvSized:
  def size(unit: String): String = "size in " + unit
  def describe: String = size("cm")

final case class OvBox(size: Int) extends OvSized
