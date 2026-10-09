package dev.teq.sbt

import java.io.File
import java.net.{InetAddress, InetSocketAddress}
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.Files
import java.security.MessageDigest
import java.util.concurrent.Executors

import scala.collection.mutable

import com.sun.net.httpserver.HttpServer
import sbt.io.IO
import sbt.util.Logger

/** A compiler's binary from its release (`Release`): a local server stands in for the release's directory, its
  * assets served through a redirect to another host name, as GitHub serves them from a CDN. */
class ReleaseSuite extends munit.FunSuite:
  private val version = "0.1.7"
  private val classifier = "linux-x86_64"
  private val asset = Release.assetName(version, classifier)
  private def hex(bytes: Array[Byte]) = bytes.map(b => f"$b%02x").mkString
  private def sha256(s: String) = hex(MessageDigest.getInstance("SHA-256").digest(s.getBytes(UTF_8)))
  private def sha1(s: String) = hex(MessageDigest.getInstance("SHA-1").digest(s.getBytes(UTF_8)))

  /** A release's directory: the binaries by classifier, the manifest and SHA256SUMS from them, either replaced. */
  private def release(dir: File, binaries: Map[String, String], manifest: Option[String] = None, sums: Option[String] = None): Unit =
    val v = new File(dir, s"v$version")
    for (c, bytes) <- binaries do IO.write(new File(v, Release.assetName(version, c)), bytes)
    val lines = binaries.toSeq.sortBy(_._1).map((c, b) => s"$c ${Release.assetName(version, c)} ${sha256(b)} ${sha1(b)} ${b.length}")
    IO.write(new File(v, s"teq-$version-binaries.txt"), manifest.getOrElse((s"teq $version 0123456789abcdef0123456789abcdef01234567" +: lines).mkString("", "\n", "\n")))
    IO.write(new File(v, "SHA256SUMS"), sums.getOrElse(binaries.toSeq.sortBy(_._1).map((c, b) => s"${sha256(b)}  ${Release.assetName(version, c)}").mkString("", "\n", "\n")))

  /** The release at `http://127.0.0.1:<port>/download`, each binary's asset (any file but the manifest and
    * SHA256SUMS) redirected to `http://localhost:<port>/objects/...`; the requests recorded as `<method> <path>`. */
  private def withRelease[A](dir: File)(body: (String, () => Seq[String]) => A): A =
    val requests = mutable.ArrayBuffer.empty[String]
    val server = HttpServer.create(new InetSocketAddress(InetAddress.getLoopbackAddress, 0), 0)
    server.setExecutor(Executors.newCachedThreadPool())
    server.createContext("/", exchange =>
      val path = exchange.getRequestURI.getPath
      requests.synchronized(requests += s"${exchange.getRequestMethod} $path")
      val name = path.substring(path.lastIndexOf('/') + 1)
      if path.startsWith("/download/") && name != "SHA256SUMS" && !name.endsWith("-binaries.txt") then
        exchange.getResponseHeaders.set("Location", s"http://localhost:${server.getAddress.getPort}/objects/${path.stripPrefix("/download/")}")
        exchange.sendResponseHeaders(302, -1)
      else
        val file = new File(dir, path.stripPrefix("/download/").stripPrefix("/objects/"))
        if file.isFile then
          val bytes = Files.readAllBytes(file.toPath)
          exchange.sendResponseHeaders(200, bytes.length)
          exchange.getResponseBody.write(bytes)
        else exchange.sendResponseHeaders(404, -1)
      exchange.close())
    server.start()
    try body(s"http://127.0.0.1:${server.getAddress.getPort}/download", () => requests.synchronized(requests.toSeq))
    finally server.stop(0)

  private val served = Served(Nil, Logger.Null)
  private def dirs() =
    val base = Files.createTempDirectory("teq-release").toFile
    (new File(base, "release"), new File(base, "cache"), base)

  test("an asset's name: teq-<version>-<classifier>, with .exe for Windows alone"):
    for c <- Seq("osx-aarch_64", "osx-x86_64", "linux-x86_64", "linux-aarch_64") do
      assertEquals(Release.assetName("0.1.7", c), s"teq-0.1.7-$c")
      assertEquals(Release.executableSuffix(c), "")
    assertEquals(Release.assetName("0.1.7", "windows-x86_64"), "teq-0.1.7-windows-x86_64.exe")
    assertEquals(Release.assetUrl(Release.DefaultBase, "0.1.7", "linux-aarch_64"), "https://github.com/Carrot-Inc/teq/releases/download/v0.1.7/teq-0.1.7-linux-aarch_64")
    assertEquals(Release.assetUrl(Release.DefaultBase + "/", "0.1.7", "windows-x86_64"), "https://github.com/Carrot-Inc/teq/releases/download/v0.1.7/teq-0.1.7-windows-x86_64.exe")
    // The manifest names each asset so: a Windows line with .exe reads, a Linux one with it does not.
    val m = Release.parseManifest(s"teq 0.1.7 c\nwindows-x86_64 teq-0.1.7-windows-x86_64.exe ${sha256("w")} ${sha1("w")} 1\n", "0.1.7", "i")
    assertEquals(m.map(_.entries.map(_.asset)), Right(Seq("teq-0.1.7-windows-x86_64.exe")))
    assert(Release.parseManifest(s"teq 0.1.7 c\nlinux-x86_64 teq-0.1.7-linux-x86_64.exe ${sha256("l")} ${sha1("l")} 1\n", "0.1.7", "i").isLeft)

  test("the releases served: from 0.1.7, by the leading <major>.<minor>.<patch>; an earlier one refused in one message"):
    for version <- Seq("0.1.0-pre.1", "0.1.0-pre.2", "0.1.1", "0.1.5", "0.1.6", "0.0.1-check", "0.1.6-check.1") do
      assertEquals(Release.floor(version), Some(s"teqVersion is $version, and releases before 0.1.7 are not served: pin 0.1.7 or later"), version)
    for version <- Seq("0.1.7", "0.1.7-check.1", "0.1.8", "0.1.60", "0.2.0", "1.0.0", "10.0.0", "99999999999999999999.0.0", "latest", "0.1") do
      assertEquals(Release.floor(version), None, version)

  test("cold: the asset fetched through its redirect, checked and kept with its receipt; warm: served from the cache with no request, the server gone"):
    val (dir, cache, base) = dirs()
    release(dir, Map(classifier -> "#!/bin/sh\necho teq 0.1.7\n", "osx-aarch_64" -> "a mac binary"))
    val url = withRelease(dir) { (url, requests) =>
      val v = Release.binary(url, version, classifier, cache, None, served, Logger.Null).fold(fail(_), identity)
      assertEquals(IO.read(v.file), "#!/bin/sh\necho teq 0.1.7\n")
      assert(v.file.canExecute)
      assertEquals(requests(), Seq(s"GET /download/v$version/teq-$version-binaries.txt", s"GET /download/v$version/SHA256SUMS",
        s"GET /download/v$version/teq-0.1.7-linux-x86_64", s"GET /objects/v$version/teq-0.1.7-linux-x86_64"))
      val receipt = IO.readLines(new File(v.file.getPath + ".receipt")).map(_.split(" ", 2)).map(a => a(0) -> a(1)).toMap
      assertEquals((receipt("sha256"), receipt("size"), receipt("version"), receipt("classifier")), (sha256("#!/bin/sh\necho teq 0.1.7\n"), "25", version, classifier))
      assertEquals(receipt("manifest"), sha256(IO.read(new File(dir, s"v$version/teq-$version-binaries.txt"))))
      url
    }
    val warm = Release.binary(url, version, classifier, cache, None, served, Logger.Null)
    assertEquals(warm.map(_.sha256), Right(sha256("#!/bin/sh\necho teq 0.1.7\n")))
    IO.delete(base)

  test("offline with no verified copy: refused, naming the release; a copy without its receipt, or a partial one, is fetched again"):
    val (dir, cache, base) = dirs()
    release(dir, Map(classifier -> "the right bytes"))
    val stopped = withRelease(dir)((url, _) => url)
    val offline = Release.binary(stopped, version, classifier, cache, None, served, Logger.Null)
    assert(offline.left.exists(_.contains(s"the release v$version of teq cannot be read")), offline.toString)
    withRelease(dir) { (url, requests) =>
      val kept = new File(Release.cacheDir(cache, url, version, classifier), asset)
      IO.write(kept, "the right bytes")
      IO.write(new File(kept.getParentFile, s".$asset.123.part"), "the rig")
      assertEquals(Release.binary(url, version, classifier, cache, None, served, Logger.Null).map(_.size), Right(15L))
      assert(requests().exists(_.endsWith(s"/objects/v$version/$asset")), "a copy without its receipt is no verified copy")
      IO.write(kept, "the righT bytes")
      kept.setLastModified(kept.lastModified + 2000)
      val before = requests().size
      assertEquals(Release.binary(url, version, classifier, cache, None, served, Logger.Null).map(_.size), Right(15L))
      assertEquals(IO.read(kept), "the right bytes")
      assert(requests().size > before, "a copy of the receipt's size that does not give its digest is fetched again")
    }
    IO.delete(base)

  test("a release without the platform's binary is refused naming the release and the classifier"):
    val (dir, cache, base) = dirs()
    release(dir, Map("osx-aarch_64" -> "a mac binary", "windows-x86_64" -> "a windows binary"))
    withRelease(dir) { (url, _) =>
      val missing = Release.binary(url, version, classifier, cache, None, served, Logger.Null)
      assertEquals(missing, Left(s"the release v$version of teq has no binary for $classifier: teq-$version-binaries.txt lists osx-aarch_64, windows-x86_64"))
      val absent = Release.binary(url, "0.1.9", classifier, cache, None, served, Logger.Null)
      assert(absent.left.exists(_.contains("the release v0.1.9 is not at")), absent.toString)
      assert(Release.manifest(url, "0.1.9", served).left.exists(_.absent))
    }
    IO.delete(base)

  test("a wrong release is refused before anything runs: another version's manifest, bytes not the manifest's, SHA256SUMS disagreeing or naming an asset twice"):
    val (dir, cache, base) = dirs()
    val bytes = Map(classifier -> "0.1.8's bytes")
    def refused(manifest: Option[String] = None, sums: Option[String] = None, served: Map[String, String] = bytes): String =
      IO.delete(dir)
      release(dir, bytes, manifest, sums)
      for (c, b) <- served do IO.write(new File(dir, s"v$version/${Release.assetName(version, c)}"), b)
      withRelease(dir)((url, _) => Release.binary(url, version, classifier, cache, None, this.served, Logger.Null)).fold(identity, v => fail(s"accepted $v"))
    assert(refused(manifest = Some(s"teq 0.1.8 0123456789abcdef0123456789abcdef01234567\n")).contains(s"not `teq $version <commit>`"))
    val swapped = refused(served = Map(classifier -> "0.1.8's bytez"))
    assert(swapped.contains("nothing of it is run"), swapped)
    assert(!cache.exists || !Files.walk(cache.toPath).anyMatch(_.getFileName.toString == asset), "nothing of a refused download is kept")
    assert(refused(sums = Some(s"${"0" * 64}  $asset\n")).contains("SHA256SUMS gives"))
    assert(refused(sums = Some(s"${sha256("0.1.8's bytes")}  $asset\n${sha256("0.1.8's bytes")} *$asset\n")).contains("2 times"))
    assert(refused(sums = Some("")).contains("does not name"))
    val twice = s"teq $version c\n$classifier $asset ${sha256("x")} ${sha1("x")} 1\n$classifier $asset ${sha256("x")} ${sha1("x")} 1\n"
    assert(refused(manifest = Some(twice)).contains("twice"))
    assert(refused(manifest = Some(s"teq $version c\n$classifier teq-0.1.8-$classifier ${sha256("x")} ${sha1("x")} 1\n")).contains("is not"))
    // A Linux binary named as Windows's is: no asset of the release.
    assert(refused(manifest = Some(s"teq $version c\n$classifier $asset.exe ${sha256("x")} ${sha1("x")} 1\n")).contains("is not"))
    IO.delete(base)

  test("a body longer than the manifest's size is cut off and refused"):
    val (dir, cache, base) = dirs()
    release(dir, Map(classifier -> "short"))
    IO.write(new File(dir, s"v$version/$asset"), "short, and then a great deal more")
    withRelease(dir) { (url, _) =>
      val refused = Release.binary(url, version, classifier, cache, None, served, Logger.Null)
      assert(refused.left.exists(_.contains("more than the 5 bytes")), refused.toString)
    }
    IO.delete(base)

  test("the lock's pin of this compiler and classifier must agree; a lock of another compiler says nothing"):
    val (dir, cache, base) = dirs()
    val b = "the binary"
    release(dir, Map(classifier -> b))
    val lock = new File(base, "teq.lock")
    def lockOf(v: String, sha: String, size: Long) =
      IO.write(lock, s"teq: $v\nformat: 1\nbinaries:\n  $classifier: https://x/teq-$v-$classifier $sha $size\n  osx-aarch_64: \"https://x/y z\" ${"a" * 40} 3\nprojects: {}\n")
    lockOf(version, sha1(b), b.length)
    assertEquals(Release.pinIn(lock, version, classifier), Some(Release.Pin(s"https://x/teq-$version-$classifier", sha1(b), b.length)))
    assertEquals(Release.pinIn(lock, "0.1.8", classifier), None)
    assertEquals(Release.pinIn(lock, version, "windows-x86_64"), None)
    withRelease(dir) { (url, _) =>
      lockOf(version, sha1(b), b.length + 1)
      val stale = Release.binary(url, version, classifier, cache, Release.pinIn(lock, version, classifier), served, Logger.Null)
      assert(stale.left.exists(_.contains("export the build again")), stale.toString)
      lockOf(version, sha1(b), b.length)
      assertEquals(Release.binary(url, version, classifier, cache, Release.pinIn(lock, version, classifier), served, Logger.Null).map(_.sha1), Right(sha1(b)))
      lockOf(version, "b" * 40, b.length)
      val warm = Release.binary(url, version, classifier, cache, Release.pinIn(lock, version, classifier), served, Logger.Null)
      assert(warm.left.exists(_.contains("export the build again")), "a verified copy is checked against the lock too")
    }
    IO.delete(base)

  test("a copy written again in place, its size and date kept, is no verified copy: refused offline, fetched again online"):
    val (dir, cache, base) = dirs()
    val good = "#!/bin/sh\necho RIGHT\n"
    release(dir, Map(classifier -> good))
    val (url, kept) = withRelease(dir)((url, _) => (url, Release.binary(url, version, classifier, cache, None, served, Logger.Null).fold(fail(_), identity).file))
    val date = Files.getLastModifiedTime(kept.toPath)
    Files.writeString(kept.toPath, "#!/bin/sh\necho WRONG\n")
    Files.setLastModifiedTime(kept.toPath, date)
    val offline = Release.binary(url, version, classifier, cache, None, served, Logger.Null)
    assert(offline.left.exists(_.contains("cannot be read")), s"the changed copy served: $offline")
    withRelease(dir) { (again, requests) =>
      val v = Release.binary(again, version, classifier, cache, None, served, Logger.Null).fold(fail(_), identity)
      assertEquals(IO.read(v.file), good)
      assert(requests().exists(_.endsWith(s"/objects/v$version/$asset")))
      // The same write to a copy the plugin runs: its digest is its bytes', not the one seen before.
      val before = Release.digestOf(v.file).map(_._2)
      val at = Files.getLastModifiedTime(v.file.toPath)
      Files.writeString(v.file.toPath, "#!/bin/sh\necho WRONG\n")
      Files.setLastModifiedTime(v.file.toPath, at)
      assertNotEquals(Release.digestOf(v.file).map(_._2), before)
    }
    IO.delete(base)

  test("a receipt short of a field is no receipt: the binary is fetched again, nothing thrown"):
    val (dir, cache, base) = dirs()
    release(dir, Map(classifier -> "the bytes"))
    withRelease(dir) { (url, requests) =>
      val v = Release.binary(url, version, classifier, cache, None, served, Logger.Null).fold(fail(_), identity)
      val receipt = new File(v.file.getPath + ".receipt")
      IO.writeLines(receipt, IO.readLines(receipt).filterNot(_.startsWith("sha1 ")))
      val before = requests().size
      assertEquals(Release.binary(url, version, classifier, cache, None, served, Logger.Null).map(_.sha1), Right(v.sha1))
      assert(requests().size > before, "fetched again")
    }
    IO.delete(base)
