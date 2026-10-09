package da

class Client(host: String = "localhost", port: Int = 80):
  def url(path: String = "/", secure: Boolean = false): String =
    (if secure then "https://" else "http://") + host + ":" + port + path
  def query[A](key: String)(value: A = null.asInstanceOf[A]): String = key + "=" + value

object Client:
  def default(retries: Int = 3): Client = new Client()
