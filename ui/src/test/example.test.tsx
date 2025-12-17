import { describe, it, expect } from 'vitest';
import { screen, renderWithQuery } from './test-utils';

/**
 * Example test to verify the testing setup works correctly.
 * Delete this file once you've confirmed the setup is working.
 */
describe('Test Setup', () => {
  it('should render a component with QueryClient provider', () => {
    renderWithQuery(<div data-testid="test-element">Hello World</div>);

    expect(screen.getByTestId('test-element')).toBeInTheDocument();
    expect(screen.getByText('Hello World')).toBeInTheDocument();
  });

  it('should have MSW handlers registered', async () => {
    // This test verifies MSW is working by making a fetch request
    const response = await fetch('/api/engines');
    const data = await response.json();

    expect(response.ok).toBe(true);
    expect(data).toEqual(['engine-1', 'engine-2', 'engine-3']);
  });
});
