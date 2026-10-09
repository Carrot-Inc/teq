package dev.teq.sbt

import java.io.{ByteArrayOutputStream, IOException, InputStream}
import java.net.URI
import java.net.http.{HttpClient, HttpRequest, HttpResponse, HttpTimeoutException}
import java.nio.ByteBuffer
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, Path, StandardOpenOption}
import java.time.Duration
import java.util.Base64
import java.util.concurrent.{CompletableFuture, CompletionStage, ExecutionException, Flow, TimeUnit, TimeoutException}

import sbt.internal.librarymanagement.ivy.IvyCredentials
import sbt.librarymanagement.Credentials
import sbt.util.Logger

/** What a repository serves, asked over HTTP: a HEAD, a GET of a small file read to its first
  * kilobytes (`limit` bytes), the whole answer within `timeout`; or a file downloaded whole
  * (`download`). A request carries the user and password sbt holds for the host it goes to, as
  * coursier sends them in sbt (`allCredentials`, a credentials file read as sbt reads it, the first
  * entry of a host taken); a redirect is followed up to five times, to another host too (a release's
  * asset is served from a CDN), never from https to plain http, each hop with its own host's. */
private[sbt] final class Served(credentials: Map[String, (String, String)], timeout: Duration):
  import Served.*

  def head(url: String): Either[String, Answer] = ask("HEAD", url, 0, BodyLimit)
  def get(url: String, limit: Int = BodyLimit): Either[String, Answer] = ask("GET", url, 0, limit)

  /** The file at `url` written whole to `target`, at most `maxBytes` of it, within `bound`: the final status,
    * or why there is none. A body past `maxBytes` fails the download, and the part written stays for the
    * caller to remove. */
  def download(url: String, target: Path, maxBytes: Long, bound: Duration): Either[String, Int] =
    val deadline = System.nanoTime + bound.toNanos
    def hop(url: String, hops: Int): Either[String, Int] =
      try
        val uri = URI.create(url)
        val left = Duration.ofNanos(deadline - System.nanoTime)
        if left.isNegative || left.isZero then return Left(s"GET $url: no whole answer within ${bound.toSeconds} s")
        val request = HttpRequest.newBuilder(uri).GET().timeout(left)
        for (user, password) <- credentials.get(uri.getHost) do
          request.header("Authorization", "Basic " + Base64.getEncoder.encodeToString(s"$user:$password".getBytes(UTF_8)))
        val response = client.send(request.build(), HttpResponse.BodyHandlers.ofInputStream())
        val location = response.headers.firstValue("Location")
        if Redirects(response.statusCode) && location.isPresent then
          response.body.close()
          val next = uri.resolve(location.get)
          if hops == MaxRedirects then Left(s"GET $url: a redirect past $MaxRedirects")
          else if uri.getScheme == "https" && next.getScheme != "https" then Left(s"GET $url redirects to $next, off https")
          else hop(next.toString, hops + 1)
        else if response.statusCode != 200 then
          response.body.close()
          Right(response.statusCode)
        else copyWithin(response.body, target, maxBytes, deadline, url).map(_ => 200)
      catch
        case e: IOException => Left(s"GET $url: ${reason(e)}")
        case e: IllegalArgumentException => Left(s"GET $url: ${reason(e)}")
        case e: InterruptedException => Left(s"GET $url: interrupted")
    hop(url, 0)

  /** The answer, or why there is none, naming the request. */
  private def ask(method: String, url: String, hops: Int, limit: Int): Either[String, Answer] =
    try
      val uri = URI.create(url)
      val request = HttpRequest.newBuilder(uri).method(method, HttpRequest.BodyPublishers.noBody())
      for (user, password) <- credentials.get(uri.getHost) do
        request.header("Authorization", "Basic " + Base64.getEncoder.encodeToString(s"$user:$password".getBytes(UTF_8)))
      val response = within(client.sendAsync(request.build(), _ => Capped(limit)))
      val length = response.headers.firstValueAsLong("Content-Length")
      val answer = Answer(response.statusCode, Option.when(length.isPresent)(length.getAsLong), new String(response.body, UTF_8))
      val location = response.headers.firstValue("Location")
      if !Redirects(answer.status) || location.isEmpty then Right(answer)
      else
        val next = uri.resolve(location.get)
        if hops == MaxRedirects then Left(s"$method $url: a redirect past $MaxRedirects")
        else if uri.getScheme == "https" && next.getScheme != "https" then Left(s"$method $url redirects to $next, off https")
        else ask(method, next.toString, hops + 1, limit)
    catch
      case e: IOException => Left(s"$method $url: ${reason(e)}")
      case e: IllegalArgumentException => Left(s"$method $url: ${reason(e)}")

  /** The response, its body read, or the exchange cancelled once `timeout` has passed. */
  private def within[A](response: CompletableFuture[A]): A =
    try response.get(timeout.toMillis, TimeUnit.MILLISECONDS)
    catch
      case _: TimeoutException =>
        response.cancel(true)
        throw new HttpTimeoutException(s"no whole answer within ${timeout.toMillis} ms")
      case e: InterruptedException =>
        response.cancel(true)
        throw e
      case e: ExecutionException => throw e.getCause

private[sbt] object Served:
  /** A repository's answer: the status, the `Content-Length` and the body's first kilobytes. */
  final case class Answer(status: Int, length: Option[Long], body: String)

  /** The requests made with the credentials sbt gives: a credentials file that cannot be read is
    * passed over with a warning, as sbt passes it over. */
  def apply(credentials: Seq[Credentials], log: Logger, timeout: Duration = Duration.ofSeconds(60)): Served =
    val direct = credentials.flatMap {
      case c: Credentials.DirectCredentials => Some(c)
      case c: Credentials.FileCredentials =>
        IvyCredentials.loadCredentials(c.path) match
          case Right(loaded) => Some(loaded)
          case Left(why) =>
            log.warn(s"teq: $why, ignoring it")
            None
    }
    new Served(direct.reverse.map(c => c.host -> (c.userName, c.passwd)).toMap, timeout)

  private val BodyLimit = 4096

  /** The body copied to `target` until its end, refused past `maxBytes` or `deadline`: a watchdog closes the
    * stream at the deadline, which ends a read that blocks. */
  private def copyWithin(in: InputStream, target: Path, maxBytes: Long, deadline: Long, url: String): Either[String, Long] =
    val watchdog = new Thread(() =>
      try
        Thread.sleep(math.max(0L, (deadline - System.nanoTime) / 1000000L))
        in.close()
      catch case _: InterruptedException => ())
    watchdog.setDaemon(true)
    watchdog.start()
    val out = Files.newOutputStream(target, StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING, StandardOpenOption.WRITE)
    try
      val buffer = new Array[Byte](1 << 16)
      var total = 0L
      var read = in.read(buffer)
      while read >= 0 do
        total += read
        if total > maxBytes then return Left(s"GET $url gives more than the $maxBytes bytes it should")
        out.write(buffer, 0, read)
        read = in.read(buffer)
      if System.nanoTime > deadline then Left(s"GET $url: no whole answer in time") else Right(total)
    catch case e: IOException =>
      Left(if System.nanoTime > deadline then s"GET $url: no whole answer in time" else s"GET $url: ${reason(e)}")
    finally
      watchdog.interrupt()
      out.close()
      in.close()
  private val MaxRedirects = 5
  private val Redirects = Set(301, 302, 303, 307, 308)

  private lazy val client = HttpClient.newBuilder()
    .connectTimeout(Duration.ofSeconds(30))
    .followRedirects(HttpClient.Redirect.NEVER)
    .version(HttpClient.Version.HTTP_1_1)
    .build()

  /** A body's first `limit` bytes: the rest is not read, the subscription cancelled once they are in. */
  private final class Capped(limit: Int) extends HttpResponse.BodySubscriber[Array[Byte]]:
    private val bytes = new ByteArrayOutputStream
    private val body = new CompletableFuture[Array[Byte]]
    @volatile private var subscription: Flow.Subscription = null
    def getBody: CompletionStage[Array[Byte]] = body
    def onSubscribe(s: Flow.Subscription): Unit =
      subscription = s
      s.request(Long.MaxValue)
    def onNext(buffers: java.util.List[ByteBuffer]): Unit =
      buffers.forEach { buffer =>
        val chunk = new Array[Byte](buffer.remaining min (limit - bytes.size))
        buffer.get(chunk)
        bytes.write(chunk)
      }
      if bytes.size >= limit && body.complete(bytes.toByteArray) then subscription.cancel()
    def onError(e: Throwable): Unit = body.completeExceptionally(e)
    def onComplete(): Unit = body.complete(bytes.toByteArray)

  /** An exception's kind and the first message along its causes (a refused connection's is its cause's). */
  private def reason(e: Throwable): String =
    val message = Iterator.iterate(e)(_.getCause).takeWhile(_ != null).map(_.getMessage).find(m => m != null && m.nonEmpty)
    e.getClass.getSimpleName + message.fold("")(": " + _)
