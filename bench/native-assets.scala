//> using file ../tools/script
// The steps of .github/workflows/native-assets.yml: the assets the native target fetches from teq's releases, each
// tag's files under a directory named by the tag. Run from the repository's root:
//
//   ./teq interp bench/native-assets.scala -- zig <dir>       upstream's zig of every host bench/zig.sh pins, under
//                                                             <dir>/zig-<version>/
//   ./teq interp bench/native-assets.scala -- publish <dir>   each tag of <dir> released with its files
//   ./teq interp bench/native-assets.scala -- compare <dir>   each tag of <dir> against what its release holds
//
// zig: each archive fetched from ziglang.org with its minisign signature, which `minisign` verifies against zig's
// public key (below, as ziglang.org/download publishes it), then held to the SHA-256 bench/zig.sh pins for it. publish: a tag
// that has no release yet is released on the commit `GITHUB_SHA` names with the files of its directory, each read
// back from its public URL and compared; a tag that has one is left as it is, whatever it holds, so that no asset is
// ever replaced. Each is a pre-release, never the latest: GitHub's latest release stays teq's, which the editor's
// extension asks for (/releases/latest) and bench/github-release.sh finds by sorting the tags of the releases
// published but pre-releases. compare: every file of each tag fetched from its public URL and compared byte for byte with the
// directory's, the release's list of assets with the directory's; a file missing on either side or of other bytes
// fails. Both print each asset's digest, size and place (`<sha256> <size> <tag>/<file>`), which the native target's
// pins of the assets are then given.
// gh asks GitHub, with GH_TOKEN; curl fetches. Status 1 on a failure, each printed as FAIL.
import java.nio.file.{Files, Path, Paths}

val repository = "Carrot-Inc/teq"
val zigKey = "RWSGOq2NVecA2UPNdBUZykf1CCb147pkmdtYxgb3Ti+JO/wCYvhbAb/U"

def sha256(bytes: Array[Byte]): String =
  java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(bytes))

def files(dir: Path): Vector[Path] =
  val s = Files.list(dir)
  try s.toArray.toVector.map(_.asInstanceOf[Path]).filter(Files.isRegularFile(_)).sortBy(_.getFileName.toString)
  finally s.close()

def dirs(dir: Path): Vector[Path] =
  val s = Files.list(dir)
  try s.toArray.toVector.map(_.asInstanceOf[Path]).filter(Files.isDirectory(_)).sortBy(_.getFileName.toString)
  finally s.close()

def row(tag: String, file: Path): String =
  val bytes = Files.readAllBytes(file)
  s"${sha256(bytes)} ${bytes.length} $tag/${file.getFileName}"

/// The bytes of an asset as a user's fetch gets them.
def fetched(tag: String, name: String, into: Path): Option[Array[Byte]] =
  val file = into.resolve(s"$tag-$name")
  val r = Sh("curl", "-fsSL", "--max-time", "600", "-o", file.toString, s"https://github.com/$repository/releases/download/$tag/$name").timeout(620).check(false).run()
  if r.ok then Some(Files.readAllBytes(file)) else None

@main def nativeAssets(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/native-assets.scala zig|publish|compare <dir>")
  val positional = args.exactly(2)
  val verb = positional(0)
  val root = Paths.get(positional(1)).toAbsolutePath.normalize
  var failures = 0
  def fail(what: String): Unit =
    println(s"FAIL $what")
    failures += 1
  verb match
    case "zig" =>
      val zigSh = Files.readAllLines(Paths.get("bench/zig.sh")).toArray.toVector.map(_.toString)
      val version = zigSh.find(_.startsWith("zig_version=")).map(_.stripPrefix("zig_version=")).getOrElse(Log.die("bench/zig.sh names no zig_version"))
      val pins = zigSh.filter(_.startsWith("zig_sha256_")).map(_.stripPrefix("zig_sha256_").split('=').toVector)
      if pins.isEmpty then Log.die("bench/zig.sh pins no zig")
      for pin <- pins do
        val (arch, sha256Pinned) = (pin(0), pin(1))
        val tag = s"zig-$version"
        val name = s"zig-$arch-linux-$version.tar.xz"
        val dir = Files.createDirectories(root.resolve(tag))
        val file = dir.resolve(name)
        val signature = root.resolve(s"$name.minisig")
        for (url, into) <- Vector(s"https://ziglang.org/download/$version/$name" -> file, s"https://ziglang.org/download/$version/$name.minisig" -> signature) do
          Sh("curl", "-fsSL", "--max-time", "600", "-o", into.toString, url).timeout(620).run()
        val verified = Sh("minisign", "-V", "-P", zigKey, "-m", file.toString, "-x", signature.toString).timeout(120).check(false).run()
        if !verified.ok then fail(s"$name: its signature is not zig's: ${verified.errTail}")
        val bytes = Files.readAllBytes(file)
        if sha256(bytes) != sha256Pinned then fail(s"$name has sha256 ${sha256(bytes)} where bench/zig.sh pins $sha256Pinned")
        println(row(tag, file))
    case "publish" =>
      val commit = Option(System.getenv("GITHUB_SHA")).filter(_.nonEmpty).getOrElse(Log.die("GITHUB_SHA names no commit"))
      val scratch = Files.createTempDirectory("native-assets.")
      Script.atExit(Sh.remove(scratch))
      for dir <- dirs(root) do
        val tag = dir.getFileName.toString
        val assets = files(dir)
        if Sh("gh", "release", "view", tag, "--repo", repository).timeout(60).check(false).run().ok then
          println(s"$tag: released already, left as it is")
        else
          Sh(Vector("gh", "release", "create", tag, "--repo", repository, "--target", commit, "--title", tag, "--prerelease", "--latest=false",
            "--notes", s"What a native build of teq fetches, made by .github/workflows/native-assets.yml at $commit.") ++
            assets.map(_.toString)*).timeout(1800).run()
          for file <- assets do
            if !fetched(tag, file.getFileName.toString, scratch).exists(java.util.Arrays.equals(_, Files.readAllBytes(file))) then
              fail(s"$tag/${file.getFileName}: its public URL does not give the bytes uploaded")
        assets.foreach(file => println(row(tag, file)))
    case "compare" =>
      val scratch = Files.createTempDirectory("native-assets.")
      Script.atExit(Sh.remove(scratch))
      for dir <- dirs(root) do
        val tag = dir.getFileName.toString
        val listed = Sh("gh", "release", "view", tag, "--repo", repository, "--json", "assets", "--jq", ".assets[].name").timeout(60).check(false).run()
        if !listed.ok then fail(s"$tag: no release (${listed.errTail})")
        else
          val published = listed.lines.filter(_.nonEmpty).toSet
          val built = files(dir).map(_.getFileName.toString).toSet
          for name <- (published -- built).toVector.sorted do fail(s"$tag/$name: published, not made again")
          for name <- (built -- published).toVector.sorted do fail(s"$tag/$name: made again, not published")
          for file <- files(dir) if published(file.getFileName.toString) do
            fetched(tag, file.getFileName.toString, scratch) match
              case None => fail(s"$tag/${file.getFileName}: its public URL gives nothing")
              case Some(bytes) if !java.util.Arrays.equals(bytes, Files.readAllBytes(file)) =>
                fail(s"$tag/${file.getFileName}: published ${sha256(bytes)} ${bytes.length}, made again ${row(tag, file)}")
              case Some(_) => println(row(tag, file))
    case other => args.fail(s"no verb $other")
  Script.exit(if failures == 0 then 0 else 1)
}
