import assert from 'node:assert/strict';
import test from 'node:test';
import { deviceLabel, groupDroboVolumes, usageAccounting, installedCapacity } from '../src/storage_presentation.ts';

test('5D Thunderbolt label preserves model and transport without an invented unit number', () => {
  assert.equal(deviceLabel('Drobo', 'Drobo 5D', 6), 'Drobo 5D - TB');
  assert.equal(deviceLabel('Drobo', '5D', 6), 'Drobo 5D - TB');
  assert.equal(deviceLabel('Drobo', 'Drobo 5D - TB', 6), 'Drobo 5D - TB');
  assert.equal(deviceLabel('Drobo', 'Drobo 5D', 1), 'Drobo 5D');
  assert.equal(deviceLabel('Drobo', 'Drobo 5N', 6), 'Drobo 5N');
});

test('both LUNs group by physical ancestry even with different driver endpoints', () => {
  const volumes = [
    { id: 'a', provider: 'drobo', enclosureId: 'pci-a', deviceId: 'lun-0', deviceName: 'Drobo 5D - TB' },
    { id: 'b', provider: 'drobo', enclosureId: 'pci-a', deviceId: 'lun-1', deviceName: 'Drobo 5D - TB' },
    { id: 'c', provider: 'drobo', enclosureId: 'pci-b', deviceId: 'lun-0b', deviceName: 'Drobo 5D - TB' },
    { id: 'local', provider: 'host', enclosureId: null, deviceId: null },
  ];
  const groups = groupDroboVolumes(volumes);
  assert.deepEqual(groups.map((group) => group.volumes.map((volume) => volume.id)), [['a', 'b'], ['c']]);
  assert.equal(groups[0].name, 'Drobo 5D - TB');
});

test('missing ancestry never merges volumes by name or assumes one enclosure', () => {
  const groups = groupDroboVolumes(['a', 'b'].map((id) => ({ id, provider: 'drobo', deviceId: null, deviceName: 'Same name' })));
  assert.equal(groups.length, 2);
  assert.equal(groups.every((group) => !group.bound), true);
});

test('same display label remains separate when source ancestry differs', () => {
  const groups = groupDroboVolumes([
    { id: 'a', provider: 'drobo', enclosureId: 'physical-a', deviceId: 'lun-a', deviceName: 'Drobo 5D - TB' },
    { id: 'b', provider: 'drobo', enclosureId: 'physical-b', deviceId: 'lun-b', deviceName: 'Drobo 5D - TB' },
  ]);
  assert.deepEqual(groups.map((group) => group.key), ['enclosure:physical-a', 'enclosure:physical-b']);
});

test('usage uses protected accounting exactly, independent of 20 TB installed and virtual 64 TiB', () => {
  assert.deepEqual(usageAccounting('11746014658560', '3754846629888', '7991168028672'), {
    total: '11746014658560', used: '3754846629888', free: '7991168028672',
  });
  assert.ok(usageAccounting('18446744073709551615', '9007199254740993', '18437736874454810622'));
  for (const values of [['0', '0', '0'], ['100', '60', '50'], ['01', '0', '1'], ['-1', '0', '0'], ['18446744073709551616', '0', '18446744073709551616']]) {
    assert.equal(usageAccounting(...values), null);
  }
});

test('unknown physical capacity remains unavailable while real empty slots stay zero', () => {
  assert.deepEqual(installedCapacity(['4000000000000', '0']), { bytes: '4000000000000', populated: 1 });
  for (const values of [[], ['4000', null], ['4000', undefined], ['4000', 'bad'], ['4000', '-1'], ['18446744073709551616']]) assert.equal(installedCapacity(values), null);
});
