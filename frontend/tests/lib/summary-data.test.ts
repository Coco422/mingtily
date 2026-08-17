import { describe, expect, test } from 'bun:test';
import { normalizeSummaryData, normalizeSummaryStatus } from '../../src/lib/summary-data';

describe('summary response normalization', () => {
  test('preserves active and interrupted statuses', () => {
    expect(normalizeSummaryStatus('PENDING')).toBe('pending');
    expect(normalizeSummaryStatus('processing')).toBe('processing');
    expect(normalizeSummaryStatus('interrupted')).toBe('interrupted');
  });

  test('does not turn missing or invalid data into a truthy empty object', () => {
    expect(normalizeSummaryData(null)).toBeNull();
    expect(normalizeSummaryData('{}')).toBeNull();
    expect(normalizeSummaryData({ markdown: '', summary_json: [] })).toBeNull();
    expect(normalizeSummaryData({ notes: { title: 'Notes', blocks: [] } })).toBeNull();
    expect(normalizeSummaryData('invalid json')).toBeNull();
  });

  test('accepts markdown and formats legacy sections', () => {
    expect(normalizeSummaryData({ markdown: '# Result' })).toEqual({ markdown: '# Result' });
    expect(normalizeSummaryData({
      _section_order: ['notes'],
      notes: { title: 'Notes', blocks: [{ content: '  item  ' }] },
    })).toEqual({
      notes: { title: 'Notes', blocks: [{ content: 'item', color: 'default' }] },
    });
  });
});
