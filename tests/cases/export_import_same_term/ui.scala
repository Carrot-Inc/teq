package lui
trait I18nHelper:
  export lcore.i18n.I18nKey as trans
  export lcore.i18n.Translate
final class Helpers extends I18nHelper
