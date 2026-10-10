//> using file ../../tools/script
// The notices of the crates the extension's WebAssembly links, integrations/zed/NOTICE-crates, which package.sh
// packs: the packages cargo resolves from Cargo.lock for the shipped target, wasm32-wasip2 (`cargo metadata
// --filter-platform`), reached from the extension by normal dependencies, a procedural macro's own left out (it
// runs in the build, nothing of it in the bundle); each with its version, its licence and the licence and notice
// files its package publishes (LICENSE*, LICENCE*, COPYING*, NOTICE*, UNLICENSE*), read from cargo's registry,
// which `cargo metadata` fills. A package that publishes none takes the files of its repository at the commit
// the package records (.cargo_vcs_info.json), kept under integrations/zed/licences/<repository>-<commit's first
// eight digits>/; without them the script stops, naming the package. A text that an earlier file of the list
// holds to the byte is named by that file, not written again. `--check` writes nothing and fails when
// the file is not what it would write (package.sh runs it before packing). Run from the repository's root:
// `./teq interp integrations/zed/notices.scala [-- --check]`.
import java.nio.file.{Files, Path, Paths}

@main def notices(argv: String*): Unit = Script.run {
  val args = new Args(argv, "integrations/zed/notices.scala [--check]")
  val check = args.flag("check")
  args.exactly(0)
  val zed = Paths.get("integrations/zed")
  if !Files.isDirectory(zed) then Log.die("integrations/zed/notices.scala runs from the repository's root")
  val metadata = Json.parse(Sh("cargo", "metadata", "--format-version", "1", "--filter-platform", "wasm32-wasip2", "--locked").in(zed).timeout(600).run().outAll())
  val packages = metadata("packages").elements.map(p => p("id").str -> p).toMap
  val nodes = metadata("resolve")("nodes").elements.map(n => n("id").str -> n).toMap
  def isMacro(id: String) = packages(id)("targets").elements.exists(_("kind").elements.exists(_.str == "proc-macro"))
  val root = metadata("resolve")("root").str
  val linked = scala.collection.mutable.LinkedHashSet.empty[String]
  def visit(id: String): Unit =
    if !isMacro(id) && linked.add(id) then
      for d <- nodes(id)("deps").elements if d("dep_kinds").elements.exists(_("kind").isNull) do visit(d("pkg").str)
  visit(root)
  linked -= root
  def listed(dir: Path): Vector[Path] =
    val s = Files.list(dir)
    try s.toArray.toVector.map(_.asInstanceOf[Path]) finally s.close()
  val noticeName = "(?i)(licen[cs]e|copying|notice|unlicense)([-._].*)?".r
  val rule = "=" * 100
  val text = new StringBuilder
  text ++= "The crates the extension's WebAssembly (extension.wasm) links, as cargo resolves them for wasm32-wasip2 from\n"
  text ++= "Cargo.lock: each with its licence and the licence and notice files it publishes. Written by\n"
  text ++= "integrations/zed/notices.scala from the registry's packages; not to be edited.\n"
  val crates = linked.toVector.map(packages).sortBy(p => (p("name").str, p("version").str))
  val seen = scala.collection.mutable.Map.empty[String, String]
  for p <- crates do
    val (name, version) = (p("name").str, p("version").str)
    val dir = Paths.get(p("manifest_path").str).getParent
    val published = listed(dir).filter(f => Files.isRegularFile(f) && noticeName.matches(f.getFileName.toString))
    val files =
      if published.nonEmpty then published
      else
        val vcs = dir.resolve(".cargo_vcs_info.json")
        val repository = p.get("repository").filterNot(_.isNull).map(_.str.stripSuffix("/").stripSuffix(".git"))
        val kept = for
          r <- repository
          if Files.isRegularFile(vcs)
        yield zed.resolve("licences").resolve(s"${r.substring(r.lastIndexOf('/') + 1)}-${Json.parse(Files.readString(vcs))("git")("sha1").str.take(8)}")
        kept.filter(Files.isDirectory(_)).map(d => listed(d).filter(Files.isRegularFile(_))).filter(_.nonEmpty).getOrElse(
          Log.die(s"$name $version publishes no licence file, and integrations/zed/licences/ holds none of its repository's${kept.fold("")(k => s" (at ${k.getFileName})")}"))
    text ++= s"\n$rule\n$name $version (${p.get("license").filterNot(_.isNull).fold("no licence named")(_.str)})\n"
    p.get("repository").filterNot(_.isNull).foreach(r => text ++= s"${r.str}\n")
    for f <- files.sortBy(_.getFileName.toString) do
      val body = Files.readString(f).replace("\r\n", "\n").stripTrailing()
      val label = s"${f.getFileName}${if published.isEmpty then s" (of its repository, at ${f.getParent.getFileName})" else ""}"
      seen.get(body) match
        case Some(first) => text ++= s"\n--- $label: the text of $first, above\n"
        case None =>
          seen(body) = s"$name $version's ${f.getFileName}"
          text ++= s"\n--- $label\n\n$body\n"
  val target = zed.resolve("NOTICE-crates")
  val written = text.toString
  if check then
    val current = if Files.exists(target) then Files.readString(target) else ""
    if current != written then
      println(s"FAIL $target is not what integrations/zed/notices.scala writes for Cargo.lock's crates (${crates.map(p => p("name").str).mkString(", ")}): run it again")
      Script.exit(1)
    println(s"notices: $target holds the ${crates.size} linked crates' notices")
  else
    Files.writeString(target, written)
    println(s"notices: wrote $target, ${crates.size} crates")
}
