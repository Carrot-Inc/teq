package dev.teq.sbt

import java.io.{File, FileInputStream, IOException}
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, StandardCopyOption}
import java.security.MessageDigest
import java.time.Duration
import java.util.concurrent.ConcurrentHashMap

import scala.util.control.NonFatal
import scala.collection.compat.*

import sbt.io.IO
import sbt.util.Logger

/** The compiler's binaries from its release (docs/TARGETS.md, "Releases"), from 0.1.7 on: under the releases'
  * base (`teqReleases`, GitHub's releases of teq), the directory `v<version>/` holds the asset
  * `teq-<version>-<classifier>` of each platform (`assetName`: `.exe` for Windows alone), `SHA256SUMS` in
  * sha256sum's format, and the binary manifest `teq-<version>-binaries.txt`, a line `teq <version> <commit>` and
  * then one `<classifier> <asset> <sha256> <sha1> <size>` a classifier, which the ship writes from the qualified
  * bytes. An asset is served through a redirect,
  * to another host; the manifest and the sums are read from the release's own directory, never from where an
  * asset's redirect leads.
  *
  * The plugin runs a binary only once its bytes are the manifest's (SHA-256, SHA-1 and size) and SHA256SUMS's,
  * and the lock's where the build's lock pins the same compiler (a lock of another compiler is stale, not an
  * authority). A verified binary is kept in the plugin's own cache, by the releases' base, the compiler's version
  * and the classifier, beside a receipt naming the digests accepted and the manifest's identity (its SHA-256): a
  * download goes to a file of its own and is renamed into place once its bytes check out, a copy without its
  * receipt, a part left by an interrupted download and a copy whose bytes no longer give the receipt's digest are
  * fetched and checked again, and only a copy that gives its receipt's digest serves offline. */
private[sbt] object Release {
  val DefaultBase = "https://github.com/Carrot-Inc/teq/releases/download"

  /** The first release served, the GitHub release's first: an earlier one is refused by the plugin, as by the
    * launchers and the Zed extension, whatever serves it. */
  val Floor = "0.1.7"

  /** Why the compiler `version` is not served: a release before the floor, the version's leading
    * `<major>.<minor>.<patch>` before 0.1.7's, whatever follows it; None for any other version, which its release
    * answers for. A SNAPSHOT published locally is no release and never asked (`TeqPlugin.resolveSnapshot`). */
  def floor(version: String): Option[String] = {
    val numbers = """^(\d+)\.(\d+)\.(\d+)""".r.findFirstMatchIn(version).toSeq.flatMap(m => (1 to 3).map(i => BigInt(m.group(i))))
    val before = numbers.zip(Seq(0, 1, 7).map(BigInt(_))).find { case (a, b) => a != b }.exists { case (a, b) => a < b }
    Option.when(before)(s"teqVersion is $version, and releases before $Floor are not served: pin $Floor or later")
  }

  /** How long a download of a binary may take, whole. */
  val DownloadBound: Duration = Duration.ofMinutes(10)

  final case class Entry(classifier: String, asset: String, sha256: String, sha1: String, size: Long)

  /** A release's binary manifest as read: its version and commit, its entries, and its identity, the SHA-256 of
    * its bytes, which a receipt records. */
  final case class Manifest(version: String, commit: String, entries: Seq[Entry], identity: String) {
    def entry(classifier: String): Option[Entry] = entries.find(_.classifier == classifier)
  }

  /** A binary checked against its release: the file in the plugin's cache and the digests it was accepted by. */
  final case class Verified(file: File, sha256: String, sha1: String, size: Long)

  /** What the build's lock pins for the classifier: its URL, SHA-1 and size. */
  final case class Pin(url: String, sha1: String, size: Long)

  def directory(base: String, version: String): String = s"${base.stripSuffix("/")}/v$version/"

  /** What a binary's name ends in on the classifier's platform: `.exe` on Windows, whose process creation looks for
    * it, nothing elsewhere; the release's assets and the plugin's copies are named by it (bench/ship-release.sh's
    * release_asset and the Zed extension's asset_name have the same rule). */
  def executableSuffix(classifier: String): String = if (classifier.startsWith("windows")) ".exe" else ""

  /** A binary's name among the release's assets: `teq-<version>-<classifier>`, `.exe` for Windows alone. */
  def assetName(version: String, classifier: String): String = s"teq-$version-$classifier${executableSuffix(classifier)}"
  def manifestName(version: String): String = s"teq-$version-binaries.txt"
  def assetUrl(base: String, version: String, classifier: String): String = directory(base, version) + assetName(version, classifier)

  private val Hex64 = "[0-9a-f]{64}".r
  private val Hex40 = "[0-9a-f]{40}".r

  /** The manifest's text, parsed and checked: the header names the version, each line names the classifier's asset
    * as the release names it, the digests and the size are well formed, and no classifier comes twice. */
  def parseManifest(text: String, version: String, identity: String): Either[String, Manifest] = {
    val lines = text.linesIterator.map(_.trim).filter(_.nonEmpty).toList
    lines match {
      case Nil => Left(s"${manifestName(version)} is empty")
      case header :: rest =>
        header.split("\\s+").toList match {
          case "teq" :: v :: commit :: Nil if v == version =>
            val parsed = rest.map { line =>
              line.split("\\s+").toList match {
                case classifier :: asset :: sha256 :: sha1 :: size :: Nil
                    if asset == assetName(version, classifier) && Hex64.pattern.matcher(sha256).matches && Hex40.pattern.matcher(sha1).matches && size.forall(_.isDigit) && size.nonEmpty =>
                  Right(Entry(classifier, asset, sha256, sha1, size.toLong))
                case _ => Left(s"${manifestName(version)}'s line '$line' is not `<classifier> <asset> <sha256> <sha1> <size>`, the asset teq-$version-<classifier> (.exe for Windows alone)")
              }
            }
            parsed.collectFirst { case Left(why) => why } match {
              case Some(why) => Left(why)
              case None =>
                val entries = parsed.collect { case Right(e) => e }
                val twice = entries.groupBy(_.classifier).collect { case (c, es) if es.size > 1 => c }
                if (twice.nonEmpty) Left(s"${manifestName(version)} names ${twice.mkString(", ")} twice")
                else Right(Manifest(version, commit, entries, identity))
            }
          case _ => Left(s"${manifestName(version)} begins '$header', not `teq $version <commit>`: it is not the release $version's")
        }
    }
  }

  /** Whether SHA256SUMS's text gives each entry's asset its SHA-256, in exactly one line. */
  def agrees(sums: String, manifest: Manifest): Either[String, Manifest] = {
    val lines = sums.linesIterator.map(_.trim).filter(_.nonEmpty).map(_.split("\\s+", 2).toList).collect {
      case digest :: name :: Nil => name.stripPrefix("*") -> digest.toLowerCase
    }.toList
    val problems = manifest.entries.flatMap { e =>
      lines.filter(_._1 == e.asset).map(_._2) match {
        case Seq(d) if d == e.sha256 => None
        case Seq(d) => Some(s"SHA256SUMS gives ${e.asset} $d, the manifest ${e.sha256}")
        case Seq() => Some(s"SHA256SUMS does not name ${e.asset}")
        case more => Some(s"SHA256SUMS names ${e.asset} ${more.size} times")
      }
    }
    if (problems.isEmpty) Right(manifest) else Left(problems.mkString("; "))
  }

  /** The release's manifest, checked against its SHA256SUMS, both read from its directory. Left with
    * `served = false` when the release is not there (the manifest answers 404), else why it cannot be read. */
  final case class Unread(why: String, absent: Boolean)

  def manifest(base: String, version: String, served: Served): Either[Unread, Manifest] = {
    val dir = directory(base, version)
    def text(name: String): Either[Unread, String] =
      served.get(dir + name, 1 << 16) match {
        case Left(why) => Left(Unread(why, absent = false))
        case Right(answer) if answer.status == 200 => Right(answer.body)
        case Right(answer) if answer.status == 404 => Left(Unread(s"the release v$version is not at $dir (GET $name answered 404)", absent = true))
        case Right(answer) => Left(Unread(s"GET $dir$name answered ${answer.status}", absent = false))
      }
    for {
      m <- text(manifestName(version))
      sums <- text("SHA256SUMS")
      parsed <- parseManifest(m, version, sha256(m.getBytes(UTF_8))).left.map(Unread(_, absent = false))
      checked <- agrees(sums, parsed).left.map(why => Unread(s"the release v$version at $dir: $why", absent = false))
    }
    yield checked
  }

  /** The cache directory of a classifier's binary of a release, under the plugin's cache. */
  def cacheDir(cache: File, base: String, version: String, classifier: String): File =
    new File(new File(new File(new File(cache, "releases"), sha256(base.stripSuffix("/").getBytes(UTF_8)).take(16)), version), classifier)

  private val locks = new ConcurrentHashMap[String, AnyRef]()

  /** The classifier's binary of the release, verified: from the cache when its copy gives its receipt's digest (no
    * request made), else fetched and checked. The lock's pin, of this compiler and classifier, must agree with
    * what is accepted. */
  def binary(base: String, version: String, classifier: String, cache: File, pin: Option[Pin], served: Served, log: Logger): Either[String, Verified] = {
    val dir = cacheDir(cache, base, version, classifier)
    val file = new File(dir, assetName(version, classifier))
    val receipt = new File(dir, assetName(version, classifier) + ".receipt")
    def pinned(v: Verified): Either[String, Verified] = pin match {
      case Some(p) if p.sha1 != v.sha1 || p.size != v.size =>
        Left(s"the build's lock pins ${p.url} as sha1 ${p.sha1} and ${p.size} bytes, and the release v$version gives ${v.sha1} and ${v.size}: export the build again (sbt teqExportAll)")
      case _ => Right(v)
    }
    locks.computeIfAbsent(dir.getAbsolutePath, _ => new AnyRef).synchronized {
      kept(file, receipt, base, version, classifier) match {
        case Some(v) => pinned(v)
        case None =>
          for {
            m <- manifest(base, version, served).left.map(u => s"the release v$version of teq cannot be read: ${u.why}")
            e <- m.entry(classifier).toRight(s"the release v$version of teq has no binary for $classifier: ${manifestName(version)} lists ${if (m.entries.isEmpty) "none" else m.entries.map(_.classifier).mkString(", ")}")
            _ <- pinned(Verified(file, e.sha256, e.sha1, e.size))
            v <- fetch(base, version, m, e, file, receipt, served, log)
          }
          yield v
      }
    }
  }

  /** The kept copy, when its receipt names this release's binary and the copy gives the receipt's digest and size;
    * a copy that does not is removed with its receipt. */
  private def kept(file: File, receipt: File, base: String, version: String, classifier: String): Option[Verified] =
    if (!file.isFile || !receipt.isFile) None
    else {
      val fields = try IO.readLines(receipt).flatMap(_.split(" ", 2) match {
        case Array(k, v) => Some(k -> v)
        case _ => None
      }).toMap catch { case _: IOException => Map.empty[String, String] }
      val wanted = Map("base" -> base.stripSuffix("/"), "version" -> version, "classifier" -> classifier)
      val ok = wanted.forall{ case (k, v) => fields.get(k).contains(v)} &&
        fields.get("size").flatMap(_.toLongOption).contains(file.length) &&
        fields.get("sha1").exists(Hex40.pattern.matcher(_).matches) &&
        fields.get("sha256").exists(d => Hex64.pattern.matcher(d).matches && digestOf(file).exists(_ == (d, fields("sha1"))))
      if (ok) Some(Verified(file, fields("sha256"), fields("sha1"), file.length))
      else {
        IO.delete(file)
        IO.delete(receipt)
        None
      }
    }

  /** The asset downloaded to a file of its own beside the copy, checked against the entry, made executable and
    * renamed into place, its receipt written after it. */
  private def fetch(base: String, version: String, m: Manifest, e: Entry, file: File, receipt: File, served: Served, log: Logger): Either[String, Verified] = {
    val url = assetUrl(base, version, e.classifier)
    try {
      IO.createDirectory(file.getParentFile)
      val part = Files.createTempFile(file.getParentFile.toPath, s".${e.asset}.", ".part")
      try {
        log.info(s"teq: fetching $url")
        served.download(url, part, e.size, DownloadBound) match {
          case Left(why) => Left(s"teq's binary for ${e.classifier} cannot be fetched from the release v$version: $why")
          case Right(status) if status != 200 => Left(s"GET $url answered $status: the release v$version does not serve its binary for ${e.classifier}")
          case Right(_) =>
            digestOf(part.toFile, cached = false) match {
              case Some((s256, s1)) if s256 == e.sha256 && s1 == e.sha1 && part.toFile.length == e.size =>
                part.toFile.setExecutable(true)
                Files.move(part, file.toPath, StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
                val text = Seq("base" -> base.stripSuffix("/"), "version" -> version, "classifier" -> e.classifier, "asset" -> e.asset,
                  "sha256" -> e.sha256, "sha1" -> e.sha1, "size" -> e.size.toString, "manifest" -> m.identity, "commit" -> m.commit)
                  .map{ case (k, v) => s"$k $v"}.mkString("", "\n", "\n")
                val receiptPart = Files.createTempFile(file.getParentFile.toPath, s".${e.asset}.", ".receipt")
                Files.write(receiptPart, text.getBytes(UTF_8))
                Files.move(receiptPart, receipt.toPath, StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
                Right(Verified(file, e.sha256, e.sha1, e.size))
              case Some((s256, s1)) =>
                Left(s"$url gave ${part.toFile.length} bytes with sha256 $s256 and sha1 $s1, and the release v$version's manifest gives ${e.size} bytes, ${e.sha256} and ${e.sha1}: nothing of it is run")
              case None => Left(s"the download of $url cannot be read")
            }
        }
      }
      finally Files.deleteIfExists(part)
    }
    catch { case NonFatal(ex) => Left(s"teq's binary for ${e.classifier} cannot be kept under ${file.getParentFile}: ${ex.getMessage}") }
  }

  /** What the lock at `lock` pins for the classifier, when it pins `version`: a lock of another compiler, or
    * none, says nothing of this one. */
  def pinIn(lock: File, version: String, classifier: String): Option[Pin] =
    if (!lock.isFile) None
    else
      try {
        val lines = IO.readLines(lock).map(_.stripSuffix("\r"))
        val pinsVersion = lines.headOption.exists(l => unquote(l.stripPrefix("teq: ")).contains(version) && l.startsWith("teq: "))
        if (!pinsVersion) None
        else {
          val block = lines.dropWhile(_ != "binaries:").drop(1).takeWhile(_.startsWith("  "))
          block.collectFirst { case l if l.startsWith(s"  $classifier: ") => l.stripPrefix(s"  $classifier: ") }
            .flatMap(unquote).map(_.split(" ").toList).collect {
              case url :: sha1 :: size :: Nil if Hex40.pattern.matcher(sha1.toLowerCase).matches && size.forall(_.isDigit) && size.nonEmpty => Pin(url, sha1.toLowerCase, size.toLong)
            }
        }
      }
      catch { case _: IOException => None }

  /** A scalar of the lock as written: plain, or double-quoted without an escape. */
  private def unquote(s: String): Option[String] =
    if (s.contains("\\")) None
    else if (s.length >= 2 && s.startsWith("\"") && s.endsWith("\"")) Some(s.drop(1).dropRight(1))
    else if (s.contains("\"")) None
    else Some(s)

  private val digests = new ConcurrentHashMap[Any, (String, String)]()

  /** What identifies a file's bytes for the session: its path, size, modification and change times and inode,
    * which a write changes even when it keeps the modification time (`cp -p`, `touch -r`), and a replacement by
    * another file too; none where the platform gives no change time or inode, and a file's bytes are then hashed at
    * every look. */
  private def identity(file: File): Option[Any] =
    try {
      val a = Files.readAttributes(file.toPath, "unix:size,lastModifiedTime,ctime,ino")
      Some((file.getAbsolutePath, a.get("size"), a.get("lastModifiedTime"), a.get("ctime"), a.get("ino")))
    }
    catch { case _: UnsupportedOperationException | _: IllegalArgumentException | _: IOException => None }

  /** A file's SHA-256 and SHA-1 from its bytes, kept for the session by its identity (`identity`) unless `cached`
    * is false. None when it cannot be read. */
  def digestOf(file: File, cached: Boolean = true): Option[(String, String)] = {
    def compute(): (String, String) = {
      val a = MessageDigest.getInstance("SHA-256")
      val b = MessageDigest.getInstance("SHA-1")
      val in = new FileInputStream(file)
      try {
        val buffer = new Array[Byte](1 << 16)
        var read = in.read(buffer)
        while (read >= 0) {
          a.update(buffer, 0, read)
          b.update(buffer, 0, read)
          read = in.read(buffer)
        }
      }
      finally in.close()
      (hex(a.digest()), hex(b.digest()))
    }
    try
      identity(file).filter(_ => cached) match {
        case None => Some(compute())
        case Some(key) => Option(digests.computeIfAbsent(key, _ => try compute() catch { case _: IOException => null }))
      }
    catch { case _: IOException => None }
  }

  def sha256(bytes: Array[Byte]): String = hex(MessageDigest.getInstance("SHA-256").digest(bytes))

  private def hex(bytes: Array[Byte]): String = bytes.map(b => f"$b%02x").mkString
}
