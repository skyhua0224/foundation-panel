// The settings data of a build that has no product data next to it (the panel on its own): empty, so the shared pages draw the label keys.
// The product's data is a module that exports the same names; vite.config.js points `@settings-data` at it when the panel is built into the product.
import { provideSettingsData } from 'foundation-ui/settings/provider.js'

export const SEARCH_INDEX = []
export const LABELS = { en: {} }

provideSettingsData({ searchIndex: SEARCH_INDEX, labels: LABELS })
