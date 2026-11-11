package fix.inlineproxy

// An inline method whose body makes an anonymous class reading a `private[pkg]` val of its trait's companion: scalac
// gives the trait an inline accessor whose body reads the val through the companion's `this` (an elidable prefix, a
// static owner's), and the class's method calls that accessor through the trait's `this` (tapir's
// `SchemaMagnoliaDerivation.derived`, its `deriveCache`).
trait ProxyReader:
  def read: Int

trait ProxyDerivation:
  inline def viaClass: Int =
    val reader = new ProxyReader { def read: Int = ProxyDerivation.hidden + 2 }
    reader.read

object ProxyDerivation:
  private[inlineproxy] val hidden: Int = 41

object proxyAuto extends ProxyDerivation
