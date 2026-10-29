import type { FilterListConfig } from '../types.js'

export type BuiltInList =
  | 'easylist'
  | 'easyprivacy'
  | 'peter-lowe'
  | 'fanboy-annoyance'
  | 'fanboy-social'
  | 'ublock-filters'
  | 'ublock-badware'
  | 'ublock-privacy'

export const DEFAULT_LISTS: Record<BuiltInList, FilterListConfig> = {
  'easylist': {
    id: 'easylist',
    name: 'EasyList',
    url: 'https://easylist.to/easylist/easylist.txt',
    enabled: true,
  },
  'easyprivacy': {
    id: 'easyprivacy',
    name: 'EasyPrivacy',
    url: 'https://easylist.to/easylist/easyprivacy.txt',
    enabled: true,
  },
  'peter-lowe': {
    id: 'peter-lowe',
    name: "Peter Lowe's Ad and tracking server list",
    url: 'https://pgl.yoyo.org/adservers/serverlist.php?hostformat=adblockplus&showintro=0&mimetype=plaintext',
    enabled: true,
  },
  'fanboy-annoyance': {
    id: 'fanboy-annoyance',
    name: 'Fanboy Annoyance List',
    url: 'https://secure.fanboy.co.nz/fanboy-annoyance.txt',
    enabled: true,
  },
  'fanboy-social': {
    id: 'fanboy-social',
    name: 'Fanboy Social Blocking List',
    url: 'https://easylist.to/easylist/fanboy-social.txt',
    enabled: true,
  },
  'ublock-filters': {
    id: 'ublock-filters',
    name: 'uBlock Origin Filters',
    url: 'https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters.txt',
    enabled: true,
  },
  'ublock-badware': {
    id: 'ublock-badware',
    name: 'uBlock Origin Badware Risks',
    url: 'https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/badware.txt',
    enabled: true,
  },
  'ublock-privacy': {
    id: 'ublock-privacy',
    name: 'uBlock Origin Privacy',
    url: 'https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/privacy.txt',
    enabled: true,
  },
}
