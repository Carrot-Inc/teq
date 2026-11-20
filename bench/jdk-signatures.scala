//> using file ../tools/script
// The JDK's signatures as teq's releases publish them under the tag jdk-signatures-<n>: the class files of the JDK's
// `lib/ct.sym` (the archive of stripped class files javac reads for --release, src/classfile/jdk.rs) for the releases
// 17 to 24 alone, in ct.sym's own layout, each directory named by the releases among those it holds (the JDK's
// `BCDEFGHIJKLMNO/` becomes `HIJKLMNO/`, a directory of none of them is left out), with the JDK's `LICENSE` (the GPL
// version 2 with the Classpath exception), its `ASSEMBLY_EXCEPTION` and its `release` file, which names the build,
// at the archive's top. The JDK is the tarball pinned below, verified before it is unpacked. Run from the
// repository's root:
//
//   ./teq interp bench/jdk-signatures.scala -- --out <dir> [--sources <dir>]
//
// The zip goes to <dir>/assets and its row of the pin table is printed. --sources names a directory that holds the
// tarball already (verified all the same); else it is fetched with curl. What it needs: curl, tar, gzip. Every entry
// carries the time 0, a DOS time before 1980, written as 1980-01-01 whatever the time zone, and the entries keep
// ct.sym's order, so that the same JDK gives the same bytes anywhere.
import java.nio.file.{Files, Path, Paths}

val tag = "jdk-signatures-1"
val first = 17
val last = 24
val jdkUrl = "https://github.com/adoptium/temurin24-binaries/releases/download/jdk-24.0.2%2B12/OpenJDK24U-jdk_x64_linux_hotspot_24.0.2_12.tar.gz"
/// Adoptium's digest of the tarball, its `.sha256.txt` beside it.
val jdkSha256 = "aea1cc55e51cf651c85f2f00ad021603fe269c4bb6493fa97a321ad770c9b096"
val jdkDir = "jdk-24.0.2+12"

def sha256(bytes: Array[Byte]): String =
  java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(bytes))

/// ct.sym's name of a release: a base-36 digit.
def releaseChar(release: Int): Char = Character.toUpperCase(Character.forDigit(release, 36))

@main def jdkSignatures(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/jdk-signatures.scala --out <dir> [--sources <dir>]")
  val out = Paths.get(args.option("out").getOrElse(args.fail("name the output directory with --out"))).toAbsolutePath.normalize
  val sources = args.option("sources").map(s => Paths.get(s).toAbsolutePath)
  args.exactly(0)
  val (dl, src, assets) = (out.resolve("dl"), out.resolve("src"), out.resolve("assets"))
  Sh.remove(src)
  for d <- Vector(dl, src, assets) do Files.createDirectories(d)

  val file = jdkUrl.substring(jdkUrl.lastIndexOf('/') + 1)
  val tarball = dl.resolve(file)
  sources.map(_.resolve(file)).filter(Files.isRegularFile(_)).foreach(kept => Files.copy(kept, tarball, java.nio.file.StandardCopyOption.REPLACE_EXISTING))
  if !Files.isRegularFile(tarball) then Sh("curl", "-fsSL", "--max-time", "600", "-o", tarball.toString, jdkUrl).timeout(620).run()
  val found = sha256(Files.readAllBytes(tarball))
  if found != jdkSha256 then Log.die(s"$jdkUrl has sha256 $found where the script pins $jdkSha256")
  val wanted = Vector("lib/ct.sym", "legal/java.base/LICENSE", "legal/java.base/ASSEMBLY_EXCEPTION", "release").map(f => s"$jdkDir/$f")
  Sh(Vector("tar", "-xzf", tarball.toString, "-C", src.toString) ++ wanted*).timeout(300).run()
  val jdk = src.resolve(jdkDir)

  val kept = (first to last).map(releaseChar).toSet
  val zip = assets.resolve(s"jdk-signatures-$first-$last.zip")
  val stream = new java.util.zip.ZipOutputStream(Files.newOutputStream(zip))
  def entry(name: String, bytes: Array[Byte]): Unit =
    val e = new java.util.zip.ZipEntry(name)
    e.setTime(0L)
    stream.putNextEntry(e)
    stream.write(bytes)
    stream.closeEntry()
  for (name, from) <- Vector("LICENSE" -> "legal/java.base/LICENSE", "ASSEMBLY_EXCEPTION" -> "legal/java.base/ASSEMBLY_EXCEPTION", "release" -> "release") do
    entry(name, Files.readAllBytes(jdk.resolve(from)))
  val ctSym = new java.util.zip.ZipFile(jdk.resolve("lib/ct.sym").toFile)
  val written = scala.collection.mutable.Set[String]()
  var classes = 0
  val entries = ctSym.entries()
  while entries.hasMoreElements do
    val e = entries.nextElement()
    val name = e.getName
    val at = name.indexOf('/')
    val releases = if at < 0 then "" else name.substring(0, at).filter(kept)
    if releases.nonEmpty then
      val renamed = releases + name.substring(at)
      if written.add(renamed) then
        entry(renamed, ctSym.getInputStream(e).readAllBytes())
        if renamed.endsWith(".sig") then classes += 1
      else if !renamed.endsWith("/") then Log.die(s"ct.sym holds $renamed twice for the releases $first to $last")
  ctSym.close()
  stream.close()
  val bytes = Files.readAllBytes(zip)
  println(s"$classes class files of the releases $first to $last")
  println(s"jdk-signatures signatures $first-$last any $tag/${zip.getFileName} ${sha256(bytes)} ${bytes.length}")
}
