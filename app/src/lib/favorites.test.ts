import { describe, expect, it } from 'vitest'
import { parseFavorites, resolveFavorites, toggleIn, type FavoriteEntry } from './favorites'

describe('parseFavorites', () => {
  it('reads a list of ids and ignores anything else', () => {
    expect(parseFavorites('["a","b"]')).toEqual(['a', 'b'])
    expect(parseFavorites('["a",2,null,"b"]')).toEqual(['a', 'b'])
  })

  it('treats missing or broken storage as no favorites', () => {
    expect(parseFavorites(null)).toEqual([])
    expect(parseFavorites('{not json')).toEqual([])
    expect(parseFavorites('{"a":1}')).toEqual([])
  })
})

describe('toggleIn', () => {
  it('appends a new id and removes one already there', () => {
    expect(toggleIn(['a'], 'b')).toEqual(['a', 'b'])
    expect(toggleIn(['a', 'b'], 'a')).toEqual(['b'])
  })
})

describe('resolveFavorites', () => {
  const entry = (id: string): FavoriteEntry => ({
    id,
    kind: 'project',
    name: id.toUpperCase(),
    href: `#/project/${id}`,
  })

  it('keeps the starring order and drops favorites that no longer exist', () => {
    const lookup = new Map([entry('a'), entry('c')].map((e) => [e.id, e]))
    expect(resolveFavorites(['c', 'gone', 'a'], lookup).map((e) => e.name)).toEqual(['C', 'A'])
  })
})
