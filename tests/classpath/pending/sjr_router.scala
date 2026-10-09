// jars: scala-library scalajs-react
// targets: js
// scalajs-react 4.0.0 from its jars: the router in the shapes a production application's router
// uses, `RouterWithPropsConfigDsl.buildRule`/`buildConfig` with `import dsl.*`, `staticRoute`, `dynamicRouteCT`,
// `caseClass` (a macro), `RoutingRule.Atom`, `pmap`, `addConditionWithFallback`, `notFound`, `renderWithP` and
// `withEffect`, resolved through `RouterLogicF.syncToPath` without a browser and rendered to markup. Checks clean since
// the fixes for `import dsl.*` with the dsl's conversions, the tags and the `caseClass` macro. A build
// stops in bodies the extra jar typed against another `DefaultEffects` (`RendererF[Function0, ..]` where teq has
// `Renderer[..]` over the bundle's), at `reverse_:::` and a local `apply` in `RoutingRulesF.fromRule`, and a route built
// with `caseClass` stops at run time in `RouteB./`, which selects `RouteB.Composition.Ato_` on `this` instead of the
// companion.
// The expectation is scalac 3.8.4's: `scala-cli --power package <file> -o out.mjs` with the directives below (the
// application's five artifacts, in its order), run by node from a directory whose parent chain has `react` and
// `react-dom` 19 in a node_modules.
//> using scala 3.8.4
//> using platform scala-js
//> using jsModuleKind es
//> using dep com.github.japgolly.scalajs-react::callback::4.0.0
//> using dep com.github.japgolly.scalajs-react::callback-ext-cats::4.0.0
//> using dep com.github.japgolly.scalajs-react::callback-ext-cats_effect::4.0.0
//> using dep com.github.japgolly.scalajs-react::core-bundle-cats_effect::4.0.0
//> using dep com.github.japgolly.scalajs-react::extra::4.0.0
package sjrrouter

import japgolly.scalajs.react.*
import japgolly.scalajs.react.callback.*
import japgolly.scalajs.react.extra.router.*
import japgolly.scalajs.react.extra.router.SetRouteVia.{HistoryPush, HistoryReplace}
import japgolly.scalajs.react.extra.router.StaticDsl.RouteB
import japgolly.scalajs.react.extra.router.StaticDsl.RouteB.literal
import japgolly.scalajs.react.vdom.html_<^.*
import scala.reflect.ClassTag

sealed trait OpenPage
object OpenPage:
  case object Home extends OpenPage
  final case class Item(id: Int, slug: Option[String]) extends OpenPage

sealed trait DeskPage
object DeskPage:
  case object Dashboard extends DeskPage
  final case class User(name: String) extends DeskPage

sealed trait Page
object Page:
  final case class Public(p: OpenPage) extends Page
  final case class Desk(p: DeskPage) extends Page

val alpha: RouteB[String] = new RouteB[String]("([a-zA-Z0-9-]+)", 1, g => Some(g(0)), identity)

def makeRule[P, Props, S <: P](path: RouteB[S], render: (S, Props) => VdomElement)(using ct: ClassTag[S]): RoutingRule[P, Props] =
  def onPage[A](f: S => A)(page: P): Option[A] = ct.unapply(page).map(f)
  val renderer = (s: S) => RendererF[japgolly.scalajs.react.util.DefaultEffects.Sync, P, Props](_ => props => render(s, props))
  RoutingRule.Atom[P, Props](
    parse = urlPath => path.route.parse(urlPath).map(Right(_)),
    path = onPage(path.route.pathFor),
    action = (_, page) => onPage(renderer)(page),
  )

val openRoutes: RoutingRule[OpenPage, Int] =
  RouterWithPropsConfigDsl[OpenPage, Int].buildRule { dsl =>
    import dsl.*
    (staticRoute(root, OpenPage.Home) ~> renderP(n => <.p(s"home $n")))
      | dynamicRouteCT((literal("/item") / int / alpha.option).caseClass[OpenPage.Item]) ~>
          dynRenderP[OpenPage.Item, VdomElement]((i, n) => <.p(s"item ${i.id} ${i.slug} $n"))
  }

val deskRoutes: RoutingRule[DeskPage, Int] =
  List[RoutingRule[DeskPage, Int]](
    makeRule[DeskPage, Int, DeskPage.User]((literal("/desk/user") / alpha).caseClass[DeskPage.User], (u, n) => <.b(s"user ${u.name} $n")),
    RouterWithPropsConfigDsl[DeskPage, Int].buildRule(dsl => dsl.staticRoute(literal("/desk").route, DeskPage.Dashboard) ~> dsl.render(<.b("dash"))),
  ).reduce(RoutingRule.Or[DeskPage, Int].apply)

val config: RouterWithPropsConfigF[CallbackTo, Page, Int] =
  RouterWithPropsConfigDsl[Page, Int]
    .buildConfig { dsl =>
      import dsl.*
      (emptyRule
        | openRoutes.pmap[Page](Page.Public.apply) { case Page.Public(p) => p }
        | deskRoutes
            .pmap[Page](Page.Desk.apply) { case Page.Desk(p) => p }
            .addConditionWithFallback(CallbackTo(true), redirectToPage(Page.Public(OpenPage.Home))(using HistoryPush)))
        .notFound(_ => redirectToPage(Page.Public(OpenPage.Home))(using HistoryReplace))
        .renderWithP((ctl, res) => n => <.main(^.className := "layout", res.renderP(n)))
    }
    .withEffect[CallbackTo]

def describe[A](cmd: RouteCmd[A]): (List[String], Option[A]) = cmd match
  case RouteCmd.Return(a)        => (Nil, Some(a))
  case RouteCmd.PushState(u)     => (List(s"push ${u.value}"), None)
  case RouteCmd.ReplaceState(u)  => (List(s"replace ${u.value}"), None)
  case RouteCmd.Log(_)           => (Nil, None)
  case RouteCmd.Sequence(init, last) =>
    val steps = init.toList.flatMap(c => describe(c)._1)
    val (lastSteps, result) = describe(last)
    (steps ++ lastSteps, result)
  case other => (List(other.toString), None)

@main def main(): Unit =
  val base = BaseUrl("http://example.com")
  val logic = new RouterLogicF(base, config)
  for path <- List("", "/item/7", "/item/7/abc", "/desk", "/desk/user/ann", "/nowhere") do
    val (steps, result) = describe(logic.syncToPath(Path(path)).runNow())
    val shown = result.fold("none")(res => s"${res.page} ${ReactDOMServer.renderToStaticMarkup(logic.render(res, 3))}")
    println(s"'$path' ${steps.mkString("[", ", ", "]")} $shown")
  println(config.rules.path(Page.Public(OpenPage.Item(5, Some("x")))))
  println(config.rules.path(Page.Desk(DeskPage.User("bob"))))
