package sbt.internal.teq

import java.io.File
import java.nio.file.Files

import sbt.internal.inc.{Analysis, CompileOutput, Incremental, JarUtils, PlainVirtualFileConverter, Stamps}
import sbt.util.Logger
import xsbti.api.*
import xsbti.compile.{CompileOrder, IncOptions, MiniOptions, MiniSetup}

import dev.teq.sbt.Json

/** The adapter over graphs teq wrote: `basic-answer.json` is teq's answer for
  * `tests/modules/basic/a` (`teq compiler build --target jvm --products ... --analysis-version 2`), its
  * source beside it. */
class ApiGraphSuite extends munit.FunSuite {
  /** The recorded answer and its source, in a directory of their own. */
  private val resources: File = {
    val dir = Files.createTempDirectory("teq-analysis-answer").toFile
    for (name <- Seq("basic-answer.json", "A.scala")) {
      val in = getClass.getResourceAsStream(s"/analysis/$name")
      try Files.copy(in, new File(dir, name).toPath)
      finally in.close()
    }
    dir
  }
  private def answer: Json.Value = Json.parse(new String(Files.readAllBytes(new File(resources, "basic-answer.json").toPath), "UTF-8"))
  private def graph: Json.Value = answer("api")

  test("a recorded graph reads into the classes it lists") {
    val file = ApiGraph.read(graph).head
    assertEquals(file.classes.map(c => (c.name, c.definitionType)).sortBy(_.toString), Seq(
      ("ma.A$package", DefinitionType.Module),
      ("ma.Base", DefinitionType.ClassDef),
      ("ma.Rect", DefinitionType.ClassDef),
      ("ma.Rect", DefinitionType.Module),
      ("ma.Util", DefinitionType.Module),
    ))
    val rect = file.classes.find(c => c.name == "ma.Rect" && c.definitionType == DefinitionType.ClassDef).get
    val declared = rect.structure.declared.map(_.name).toSet
    assert(declared("copy") && declared("ma;Rect;init;") && declared("productArity"), declared)
    assertEquals(rect.structure.parents.collect { case p: Projection => p.id }.toSeq, Seq("Serializable", "Product", "Equals", "Object", "Matchable", "Any"))
  }

  test("writing a graph read back gives the same graph") {
    val once = ApiGraph.write(ApiGraph.read(graph))
    val twice = ApiGraph.write(ApiGraph.read(Json.parse(once)))
    assertEquals(twice, once)
  }

  test("a node referred to twice is one object") {
    val g = Json.parse("""{"nodes":[
      ["Singleton",[["Id","p"],["This"]]],
      ["Projection",0,"A"],
      ["Structure",[1,1],[],[]],
      ["Def","f","Public",0,[],[],[],2],
      ["Def","g","Public",0,[],[],[],2],
      ["EmptyType"],
      ["Structure",[1],[3,4],[]],
      ["ClassLike","p.C","Public",0,[],"ClassDef",5,6,[],[],true,[]]
    ],"files":[{"file":"A.scala","classes":[7],"products":[],"local":[]}]}""")
    val c = ApiGraph.read(g).head.classes.head
    val Array(f: Def, h: Def) = c.structure.declared: @unchecked
    assert(f.returnType eq h.returnType)
    assert(f.returnType.isInstanceOf[Structure])
  }

  test("a structure may refer to itself") {
    val g = Json.parse("""{"nodes":[
      ["Structure",[0],[],[]],
      ["Val","x","Public",0,[],0],
      ["EmptyType"],
      ["Structure",[],[1],[]],
      ["ClassLike","p.C","Public",0,[],"ClassDef",2,3,[],[],true,[]]
    ],"files":[{"file":"A.scala","classes":[4],"products":[],"local":[]}]}""")
    val v = ApiGraph.read(g).head.classes.head.structure.declared.head.asInstanceOf[Val]
    val s = v.tpe.asInstanceOf[Structure]
    assert(s.parents.head eq s)
  }

  test("a node that refers to itself eagerly is refused, not followed") {
    for (nodes <- Seq(
        """[["Projection",0,"X"]]""",
        """[["Parameterized",1,[]],["Annotated",0,[]]]""",
        """[["EmptyType"],["Structure",[],[2],[]],["Def","f","Public",0,[],[],[],3],["Constant",3,"c"]]""",
      ))
    {
      val g = Json.parse(s"""{"nodes":$nodes,"files":[]}""")
      val e = intercept[ApiGraph.Malformed](ApiGraph.read(g))
      assert(e.getMessage.contains("refers to itself"), e.getMessage)
    }
  }

  test("an unknown node is refused with its name") {
    val g = Json.parse("""{"nodes":[["Mystery"]],"files":[{"file":"A.scala","classes":[0],"products":[],"local":[]}]}""")
    val e = intercept[ApiGraph.Malformed](ApiGraph.read(g))
    assert(e.getMessage.contains("Mystery"), e.getMessage)
  }

  test("zinc hashes the adapter's classes by name") {
    val analysis = throughZinc(answer)
    val rect = analysis.apis.internal("ma.Rect")
    val names = rect.nameHashes.map(_.name).toSet
    assert(names("copy") && names("apply") && names("Rect") && names("fromProduct"), names)
    assert(rect.api.classApi.structure.declared.nonEmpty && rect.api.objectApi.structure.declared.nonEmpty)
    assertEquals(analysis.apis.internal.keySet, Set("ma.A$package", "ma.Base", "ma.Rect", "ma.Util"))
    val products = analysis.relations.productClassName._2s.toSet
    assert(products("ma.Rect$") && products("ma.Rect"), products)
  }

  test("an answer of another version is refused") {
    val other = Json.parse("""{"ok":true,"analysisVersion":4,"analysis":[],"api":{"nodes":[],"files":[]}}""")
    val e = intercept[ApiGraph.Malformed](TeqAnalysis.feed(other, resources, resources, null, PlainVirtualFileConverter.converter))
    assert(e.getMessage.contains("analysisVersion 4"), e.getMessage)
  }

  /** A jar the dependencies name, which zinc stamps. */
  private val lib: File = {
    val f = new File(resources, "lib.jar")
    Files.write(f.toPath, Array[Byte]())
    f
  }

  /** The recorded answer as version 3, its file's dependencies `deps` and the answer's `entries`. */
  private def withDeps(deps: String, entries: String = s"""["${lib.getPath}","jrt:/modules/java.base/java/lang/Object.class"]"""): Json.Value = {
    val Json.Obj(fields) = answer: @unchecked
    val Json.Obj(api) = fields("api"): @unchecked
    val files = api("files").items.map { case Json.Obj(f) => Json.Obj(f.updated("deps", Json.parse(deps))); case other => other }
    Json.Obj(fields.updated("analysisVersion", Json.Num("3")).updated("api", Json.Obj(api.updated("files", Json.Arr(files)).updated("entries", Json.parse(entries)))))
  }

  private def basicDeps = """{
    "ma.Util":{"names":["Base"],"patmat":["Expr"],"classes":[["ma.Base","DependencyByMemberRef"]],
      "binaries":[[0,"lib.C","DependencyByMemberRef"],[0,"lib.B","DependencyByMemberRef"],[1,"java.lang.Object","DependencyByInheritance"]]},
    "ma.Rect":{"names":[],"patmat":[],"classes":[],"binaries":[]}}"""

  test("a version-3 answer's dependencies reach the callback after every file's API and before the products") {
    val calls = scala.collection.mutable.ArrayBuffer.empty[(String, Seq[Any])]
    val callback = java.lang.reflect.Proxy.newProxyInstance(getClass.getClassLoader, Array(classOf[xsbti.AnalysisCallback]), (_, m, args) => {
      calls += ((m.getName, Option(args).fold(Seq.empty[Any])(_.toSeq)))
      null
    }).asInstanceOf[xsbti.AnalysisCallback]
    TeqAnalysis.feed(withDeps(basicDeps), resources, resources, callback, PlainVirtualFileConverter.converter)
    val kinds = calls.map(_._1)
    val firstDep = kinds.indexWhere(k => k == "usedName" || k == "classDependency" || k == "binaryDependency")
    assert(kinds.lastIndexOf("api") < firstDep && kinds.indexOf("generatedNonLocalClass") > kinds.lastIndexWhere(k => k == "usedName" || k == "classDependency" || k == "binaryDependency"), kinds)
    val used = calls.collect { case ("usedName", Seq(c, n, s: java.util.EnumSet[?])) => (c.toString, n.toString, s.toArray.map(_.toString).toSet) }
    assertEquals(used.toSet, Set(("ma.Util", "Base", Set("Default")), ("ma.Util", "Expr", Set("Default", "PatMatTarget"))))
    assertEquals(calls.collect { case ("classDependency", args) => args.map(_.toString) }.toSeq, Seq(Seq("ma.Base", "ma.Util", "DependencyByMemberRef")))
    val binaries = calls.collect { case ("binaryDependency", Seq(entry: java.nio.file.Path, binary, from, _, context)) => (entry.getFileSystem.provider.getScheme, entry.toString, binary.toString, from.toString, context.toString) }
    assertEquals(binaries.toSet, Set(
      ("file", lib.getPath, "lib.C", "ma.Util", "DependencyByMemberRef"),
      ("file", lib.getPath, "lib.B", "ma.Util", "DependencyByMemberRef"),
      ("jrt", "/modules/java.base/java/lang/Object.class", "java.lang.Object", "ma.Util", "DependencyByInheritance"),
    ))
  }

  test("zinc stores a version-3 answer's dependencies as its relations") {
    val analysis = throughZinc(withDeps(basicDeps))
    assertEquals(analysis.relations.internalClassDep.forward("ma.Util"), Set("ma.Base"))
    val source = analysis.relations.classes.reverse("ma.Util").head
    val jar = analysis.relations.libraryDep.forward(source).find(_.id == lib.toPath.toString)
    assert(jar.nonEmpty, analysis.relations.libraryDep.forward(source))
    // zinc keeps one class per library, the first by name.
    assertEquals(analysis.relations.libraryClassName.forward(jar.get), Set("lib.B"))
    assertEquals(sbt.internal.inc.StoredNames.of(analysis)("ma.Util"), Set(("Base", Set("Default")), ("Expr", Set("Default", "PatMatTarget"))))
  }

  test("a version-3 answer's dependencies of the wrong shape are refused before any callback") {
    val mutations = Seq(
      "an answer without its entries" -> ("entries", {
        val Json.Obj(fields) = withDeps("{}"): @unchecked
        val Json.Obj(api) = fields("api"): @unchecked
        Json.Obj(fields.updated("api", Json.Obj(api - "entries")))
      }),
      "a binary's entry out of the table" -> ("2 is no entry", withDeps("""{"ma.Util":{"names":[],"patmat":[],"classes":[],"binaries":[[2,"a.C","DependencyByMemberRef"]]}}""")),
      "a binary's entry a path" -> ("not an index", withDeps("""{"ma.Util":{"names":[],"patmat":[],"classes":[],"binaries":[["a.jar","a.C","DependencyByMemberRef"]]}}""")),
      "a name twice in patmat" -> ("twice", withDeps("""{"ma.Util":{"names":[],"patmat":["x","x"],"classes":[],"binaries":[]}}""")),
      "a file without its dependencies" -> ("carries no dependencies", {
        val Json.Obj(fields) = withDeps("{}"): @unchecked
        val Json.Obj(api) = fields("api"): @unchecked
        Json.Obj(fields.updated("api", Json.Obj(api.updated("files", Json.Arr(api("files").items.map { case Json.Obj(f) => Json.Obj(f - "deps"); case o => o })))))
      }),
      "dependencies of a class the file does not define" -> ("ma.Other", withDeps("""{"ma.Other":{"names":[],"patmat":[],"classes":[],"binaries":[]}}""")),
      "dependencies that are an array" -> ("not an object", withDeps("""[]""")),
      "a class's dependencies without their binaries" -> ("names, patmat, classes and binaries", withDeps("""{"ma.Util":{"names":[],"patmat":[],"classes":[]}}""")),
      "a name that is no string" -> ("a name of names", withDeps("""{"ma.Util":{"names":[["x",["Default"]]],"patmat":[],"classes":[],"binaries":[]}}""")),
      "a name twice" -> ("twice", withDeps("""{"ma.Util":{"names":["x","x"],"patmat":[],"classes":[],"binaries":[]}}""")),
      "a context no enum has" -> ("ByAccident", withDeps("""{"ma.Util":{"names":[],"patmat":[],"classes":[["ma.Base","ByAccident"]],"binaries":[]}}""")),
      "a class dependency twice" -> ("twice", withDeps("""{"ma.Util":{"names":[],"patmat":[],"classes":[["ma.Base","DependencyByMemberRef"],["ma.Base","DependencyByMemberRef"]],"binaries":[]}}""")),
      "a binary dependency without its context" -> ("3", withDeps("""{"ma.Util":{"names":[],"patmat":[],"classes":[],"binaries":[[0,"a.C"]]}}""")),
      "a binary dependency's name a number" -> ("binary class name", withDeps("""{"ma.Util":{"names":[],"patmat":[],"classes":[],"binaries":[[0,5,"DependencyByMemberRef"]]}}""")),
      "a version-2 answer with dependencies" -> ("version 2", {
        val Json.Obj(fields) = withDeps("{}"): @unchecked
        Json.Obj(fields.updated("analysisVersion", Json.Num("2")))
      }),
    )
    for ((what, (part, mutated)) <- mutations) {
      val (message, calls) = refusal(mutated)
      assert(message.contains(part), s"$what: $message")
      assertEquals(calls, 0, what)
    }
  }

  test("a second file's malformed dependencies refuse the whole answer before any callback") {
    val (message, calls) = refusal("""{"ok":true,"analysisVersion":3,"analysis":[],"api":{"entries":[],"nodes":[
      ["EmptyType"],
      ["Structure",[],[],[]],
      ["ClassLike","p.A","Public",0,[],"ClassDef",0,1,[],[],true,[]],
      ["ClassLike","p.B","Public",0,[],"ClassDef",0,1,[],[],true,[]]
    ],"files":[
      {"file":"A.scala","classes":[2],"products":[["p.A","p.A"]],"local":[],"deps":{"p.A":{"names":["B"],"patmat":[],"classes":[["p.B","DependencyByMemberRef"]],"binaries":[]}}},
      {"file":"B.scala","classes":[3],"products":[["p.B","p.B"]],"local":[],"deps":{"p.B":{"names":[],"patmat":[],"classes":[["p.A","Sometimes"]],"binaries":[]}}}
    ]}}""")
    assert(message.contains("Sometimes"), message)
    assertEquals(calls, 0)
  }

  test("an answer without a graph is refused") {
    val e = intercept[ApiGraph.Malformed](TeqAnalysis.feed(Json.parse("""{"ok":true,"analysis":[]}"""), resources, resources, null, PlainVirtualFileConverter.converter))
    assert(e.getMessage.contains("no analysis graph"), e.getMessage)
  }

  /** What `answer` hands a callback that counts its calls, refused or not. */
  private def refusal(answer: String): (String, Int) = refusal(Json.parse(answer))

  private def refusal(answer: Json.Value): (String, Int) = {
    var calls = 0
    val callback = java.lang.reflect.Proxy.newProxyInstance(getClass.getClassLoader, Array(classOf[xsbti.AnalysisCallback]), (_, _, _) => { calls += 1; null }).asInstanceOf[xsbti.AnalysisCallback]
    val e = intercept[ApiGraph.Malformed](TeqAnalysis.feed(answer, resources, resources, callback, PlainVirtualFileConverter.converter))
    (e.getMessage, calls)
  }

  /** The recorded answer with node `i` replaced. */
  private def withNode(i: Int, node: Json.Value): Json.Value = {
    val Json.Obj(fields) = answer: @unchecked
    val Json.Obj(api) = fields("api"): @unchecked
    Json.Obj(fields.updated("api", Json.Obj(api.updated("nodes", Json.Arr(api("nodes").items.updated(i, node))))))
  }

  private def replaced(node: Json.Value, field: Int, value: Json.Value): Json.Value = Json.Arr(node.items.updated(field, value))

  test("a failed build's answer is refused, not taken for an empty compile") {
    val (message, calls) = refusal("""{"ok":false,"analysisVersion":2}""")
    assert(message.contains("failed build"), message)
    assertEquals(calls, 0)
  }

  test("an answer whose graph teq could not state is refused with its errors") {
    val (message, calls) = refusal("""{"ok":true,"analysisVersion":2,"analysis":[],"apiErrors":["unpicklable"]}""")
    assert(message.contains("unpicklable"), message)
    assertEquals(calls, 0)
  }

  test("an answer of version 2 without its graph is refused") {
    val (message, calls) = refusal("""{"ok":true,"analysisVersion":2,"analysis":[]}""")
    assert(message.contains("no analysis graph (api)"), message)
    assertEquals(calls, 0)
  }

  test("a second file's product without its class refuses the whole answer before any callback") {
    val (message, calls) = refusal("""{"ok":true,"analysisVersion":2,"analysis":[],"api":{"nodes":[
      ["EmptyType"],
      ["Structure",[],[],[]],
      ["ClassLike","p.A","Public",0,[],"ClassDef",0,1,[],[],true,[]],
      ["ClassLike","p.B","Public",0,[],"ClassDef",0,1,[],[],true,[]]
    ],"files":[
      {"file":"A.scala","classes":[2],"products":[["p.A","p.A"]],"local":[]},
      {"file":"B.scala","classes":[3],"products":[["p.Missing","p.Missing"]],"local":[]}
    ]}}""")
    assert(message.contains("p.Missing"), message)
    assertEquals(calls, 0)
  }

  test("a discovery entry of the wrong shape is refused before any callback") {
    val (message, calls) = refusal("""{"ok":true,"analysisVersion":2,"analysis":[{"file":"A.scala","local":null,"classes":[]}],"api":{"nodes":[],"files":[]}}""")
    assert(message.contains("local classes"), message)
    assertEquals(calls, 0)
  }

  test("a populated graph with a field of the wrong type is refused before any callback") {
    val nodes = answer("api")("nodes").items
    val util = nodes.indexWhere(n => n.items.head == Json.Str("ClassLike") && n.items(1) == Json.Str("ma.Util") && n.items(5) == Json.Str("Module"))
    val structure = nodes(util).items(7).str.toInt
    val twice = nodes(structure).items(2).items.map(_.str.toInt).find(d => nodes(d).items(1) == Json.Str("twice")).get
    val parameters = nodes(twice).items(6)
    val mutations = Seq(
      "a structure of nulls" -> withNode(structure, Json.Arr(Seq(Json.Str("Structure"), Json.Null, Json.Null, Json.Null))),
      "a structure whose declarations are null" -> withNode(structure, replaced(nodes(structure), 2, Json.Null)),
      "a structure whose parents are an object" -> withNode(structure, replaced(nodes(structure), 1, Json.Obj(Map.empty))),
      "a definition's name a number" -> withNode(twice, replaced(nodes(twice), 1, Json.Num("5"))),
      "a definition's access malformed" -> withNode(twice, replaced(nodes(twice), 2, Json.Arr(Seq(Json.Str("Private"))))),
      "a parameter list's implicit flag a string" -> withNode(twice, replaced(nodes(twice), 6, Json.Arr(parameters.items.map(l => Json.Arr(Seq(l.items.head, Json.Str("false"))))))),
      "a definition missing its result" -> withNode(twice, Json.Arr(nodes(twice).items.dropRight(1))),
      "a definition with a field too many" -> withNode(twice, Json.Arr(nodes(twice).items :+ Json.Null)),
      "a class's top-level flag null" -> withNode(util, replaced(nodes(util), 10, Json.Null)),
      "a class's definition type unknown" -> withNode(util, replaced(nodes(util), 5, Json.Str("Object"))),
    )
    for ((what, mutated) <- mutations) {
      val (message, calls) = refusal(mutated)
      assert(message.nonEmpty, what)
      assertEquals(calls, 0, what)
    }
  }

  test("a graph of the wrong shape is refused before any callback") {
    for ((answer, part) <- Seq(
        """{"ok":true,"analysisVersion":2,"analysis":[],"api":{"files":[]}}""" -> "nodes",
        """{"ok":true,"analysisVersion":2,"analysis":[],"api":{"nodes":[]}}""" -> "files",
        """{"ok":true,"analysisVersion":2,"analysis":[],"api":{"nodes":[],"files":[{"file":"A.scala","classes":[0],"products":[],"local":[]}]}}""" -> "no node 0",
        """{"ok":true,"analysisVersion":2,"analysis":[],"api":{"nodes":[],"files":[{"file":"A.scala","classes":[],"products":[["p.C"]],"local":[]}]}}""" -> "product",
        """{"ok":true,"analysisVersion":2,"analysis":[],"api":{"nodes":[],"files":[{"file":"A.scala","classes":[]}]}}""" -> "products",
        """{"ok":true,"analysisVersion":2,"analysis":[],"api":{"nodes":[["EmptyType"],["Structure",[7],[],[]],["ClassLike","p.C","Public",0,[],"ClassDef",0,1,[],[],true,[]]],"files":[{"file":"A.scala","classes":[2],"products":[],"local":[]}]}}""" -> "no node 7",
      ))
    {
      val (message, calls) = refusal(answer)
      assert(message.contains(part), s"$part: $message")
      assertEquals(calls, 0)
    }
  }

  /** The answer through the adapter and zinc's incremental compiler, which calls it once. */
  private def throughZinc(answer: Json.Value): Analysis = {
    val converter = PlainVirtualFileConverter.converter
    val classes = Files.createTempDirectory("teq-analysis").toFile
    val sources = answer("api")("files").items.map(f => converter.toVirtualFile(TeqAnalysis.absolute(resources, f("file").str).toPath)).toSet
    val output = CompileOutput(classes.toPath)
    val setup = MiniSetup.of(output, MiniOptions.of(Array(), Array(), Array()), "teq", CompileOrder.Mixed, true, Array())
    var thrown: Option[Throwable] = None
    val (_, analysis) = Incremental.apply(
      sources, converter, TeqAnalysis.noLookup, Analysis.empty, IncOptions.of().withApiDebug(true), setup, Stamps.timeWrapBinaryStamps(converter), output,
      JarUtils.createOutputJarContent(output), None, None, None, Logger.Null,
    ) { case (_, _, callback, _) =>
      try TeqAnalysis.feed(answer, resources, classes, callback, converter)
      catch { case e: Throwable => thrown = Some(e) }
    }
    thrown.foreach(throw _)
    analysis
  }
}
