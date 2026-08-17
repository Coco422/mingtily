import { describe, expect, test } from 'bun:test';
import { prefixResolvedSpeaker, resolveSpeaker } from '../../src/lib/speaker-map';

const participants = [{
  id: '25c720ea-3d8d-4c52-ae19-8bfe3e462e95',
  name: '张三',
  sourceSpeakers: ['speaker_00', 'speaker_01'],
}];

describe('meeting speaker resolver', () => {
  test('merged source labels share the same name and color', () => {
    const first = resolveSpeaker('speaker_00', participants);
    const second = resolveSpeaker('speaker_01', participants);
    expect(first?.label).toBe('张三');
    expect(second?.label).toBe('张三');
    expect(first?.color).toBe(second?.color);
  });

  test('summary and copy prefixes use the resolved name', () => {
    expect(prefixResolvedSpeaker('你好', 'speaker_01', participants)).toBe('张三: 你好');
  });

  test('unmapped labels keep their detected identity', () => {
    expect(resolveSpeaker('speaker_02', participants)?.label).toBe('Speaker 3');
  });
});
