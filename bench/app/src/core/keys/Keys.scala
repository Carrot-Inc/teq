package meridian.core.keys

/** An identifier that wraps a number: a one-field case class that `derives LongKey`, or an
  * opaque type whose companion extends [[LongKey.Opaque]]. The instance is built by a macro
  * from the class, so that the name on the wire and in messages is the type's own. */
trait LongKey[K]:
  def keyName: String
  def wrap(raw: Long): K
  extension (k: K) def raw: Long
  def parse(text: String): Either[String, K] = text.toLongOption.map(wrap).toRight(s"invalid $keyName: $text")

object LongKey:
  transparent inline def derived[K]: LongKey[K] = ${ KeyMacros.longKey[K] }
  inline def apply[K](using key: LongKey[K]): key.type = key
  def of[K](name: String, make: Long => K, read: K => Long): LongKey[K] = new LongKey[K]:
    def keyName = name
    def wrap(raw: Long) = make(raw)
    extension (k: K) def raw: Long = read(k)

  /** The companion of `opaque type K = Long`: `object K extends LongKey.Opaque[K]`. */
  trait Opaque[K]:
    inline given key: LongKey[K] = LongKey.derived[K]
    def apply(raw: Long): K = raw.asInstanceOf[K]
    def unapply(k: K): Some[Long] = Some(k.asInstanceOf[Long])

/** The text counterpart of [[LongKey]]. */
trait TextKey[K]:
  def keyName: String
  def wrap(raw: String): K
  extension (k: K) def raw: String
  def parse(text: String): Either[String, K] = if text.isEmpty then Left(s"empty $keyName") else Right(wrap(text))

object TextKey:
  transparent inline def derived[K]: TextKey[K] = ${ KeyMacros.textKey[K] }
  inline def apply[K](using key: TextKey[K]): key.type = key
  def of[K](name: String, make: String => K, read: K => String): TextKey[K] = new TextKey[K]:
    def keyName = name
    def wrap(raw: String) = make(raw)
    extension (k: K) def raw: String = read(k)

  trait Opaque[K]:
    inline given key: TextKey[K] = TextKey.derived[K]
    def apply(raw: String): K = raw.asInstanceOf[K]
    def unapply(k: K): Some[String] = Some(k.asInstanceOf[String])
