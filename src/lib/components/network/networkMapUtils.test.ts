import { describe, expect, it } from 'vitest';

import { buildNodePopupModel, buildServicePopupModel, type NMNode } from './networkMapUtils';

describe('network map popup models', () => {
  it('includes trace, impact, and inspect actions for risky nodes', () => {
    const node: NMNode = {
      id: 'node-risky-1',
      name: 'POP Cileungsi',
      node_type: 'pop',
      status: 'degraded',
      lat: -6.2,
      lng: 106.8,
      metadata: {
        impacted_services: 6,
      },
    };

    const model = buildNodePopupModel(node);

    expect(model.actions.map((action) => action.key)).toEqual(
      expect.arrayContaining(['trace', 'impact', 'inspect']),
    );
    expect(model.impactText).toContain('6');
  });

  it('prioritizes service status and quick actions for service-backed nodes', () => {
    const serviceNode: NMNode = {
      id: 'svc-node-1',
      name: 'Service 20 Mbps',
      node_type: 'customer_premise',
      status: 'active',
      lat: -6.21,
      lng: 106.82,
      metadata: {
        service_id: 'svc-20',
        service_name: 'Internet 20 Mbps',
        service_type: 'pppoe',
        service_label: 'PPPoE Bronze',
        customer_name: 'Budi Santoso',
      },
    };

    const model = buildServicePopupModel(serviceNode);

    expect(model.kicker).toBe('Service');
    expect(model.title).toContain('Internet 20 Mbps');
    expect(model.subtitle).toContain('Budi Santoso');
    expect(model.actions.map((action) => action.key)).toEqual(
      expect.arrayContaining(['inspect', 'trace', 'impact']),
    );
  });
});
