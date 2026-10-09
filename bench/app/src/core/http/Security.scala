package meridian.core.http

/** The credentials a private endpoint takes: a token and the site the caller works in. */
final case class Credentials(token: String, siteKey: Option[String]):
  def scoped(site: String): Credentials = copy(siteKey = Some(site))

object Security:
  val credentials: Input[Credentials] =
    (header[String]("X-Token") / header[Option[String]]("X-Site-Key")).mapTo(t => Credentials(t._1, t._2))(c => (c.token, c.siteKey))
  val anonymous: Credentials = Credentials("", None)
