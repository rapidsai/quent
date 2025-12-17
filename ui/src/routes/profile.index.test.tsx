import { describe, it, expect, beforeEach } from 'vitest';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/mocks/server';
import { screen, renderWithRouter, waitFor, fireEvent } from '@/test/test-utils';

const API_BASE = 'http://localhost:8000/api';

describe('EngineSelectionPage', () => {
  beforeEach(() => {
    // Set up default handlers for the profile page API endpoints
    server.use(
      http.get(`${API_BASE}/engine/list`, () => {
        return HttpResponse.json(['engine-1', 'engine-2', 'engine-3']);
      }),
      http.get(`${API_BASE}/engine/:engineId/query_groups`, ({ params }) => {
        const { engineId } = params;
        return HttpResponse.json([`${engineId}-coordinator-1`, `${engineId}-coordinator-2`]);
      }),
      http.get(`${API_BASE}/engine/:engineId/query_groups/:coordinatorId/list_queries`, () => {
        return HttpResponse.json(['query-1', 'query-2', 'query-3']);
      })
    );
  });

  describe('Page rendering', () => {
    it('renders the page title and description', async () => {
      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: /query profiler/i })).toBeInTheDocument();
      });
      expect(screen.getByText(/select an engine, coordinator, and query/i)).toBeInTheDocument();
    });

    it('renders all three select dropdowns with labels', async () => {
      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByText('Engine')).toBeInTheDocument();
      });
      expect(screen.getByText('Coordinator')).toBeInTheDocument();
      expect(screen.getByText('Query')).toBeInTheDocument();
    });

    it('renders engine select with placeholder', async () => {
      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByText('Select Engine')).toBeInTheDocument();
      });
    });

    it('renders coordinator select with placeholder', async () => {
      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByText('Select Coordinator')).toBeInTheDocument();
      });
    });

    it('renders query select with placeholder', async () => {
      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByText('Select Query')).toBeInTheDocument();
      });
    });
  });

  describe('Initial visibility state', () => {
    it('hides coordinator dropdown initially', async () => {
      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: /query profiler/i })).toBeInTheDocument();
      });

      // Coordinator section should be invisible
      const coordinatorLabel = screen.getByText('Coordinator');
      expect(coordinatorLabel.parentElement).toHaveClass('invisible');
    });

    it('hides query dropdown initially', async () => {
      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: /query profiler/i })).toBeInTheDocument();
      });

      // Query section should be invisible
      const queryLabel = screen.getByText('Query');
      expect(queryLabel.parentElement).toHaveClass('invisible');
    });

    it('shows engine dropdown initially', async () => {
      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: /query profiler/i })).toBeInTheDocument();
      });

      // Engine section should be visible (no invisible class)
      const engineLabel = screen.getByText('Engine');
      expect(engineLabel.parentElement).not.toHaveClass('invisible');
    });
  });

  describe('API data fetching', () => {
    it('loads engines from API', async () => {
      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByText('Select Engine')).toBeInTheDocument();
      });

      // Open the dropdown using fireEvent
      const trigger = screen.getByText('Select Engine');
      fireEvent.click(trigger);

      // Verify engines are displayed
      await waitFor(() => {
        expect(screen.getByText('engine-1')).toBeInTheDocument();
      });
      expect(screen.getByText('engine-2')).toBeInTheDocument();
      expect(screen.getByText('engine-3')).toBeInTheDocument();
    });

    it('shows empty state when no engines are available', async () => {
      server.use(
        http.get(`${API_BASE}/engine/list`, () => {
          return HttpResponse.json([]);
        })
      );

      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByText('Select Engine')).toBeInTheDocument();
      });

      const trigger = screen.getByText('Select Engine');
      fireEvent.click(trigger);

      await waitFor(() => {
        expect(screen.getByText(/no engines available/i)).toBeInTheDocument();
      });
    });

    it('handles API error gracefully when fetching engines fails', async () => {
      server.use(
        http.get(`${API_BASE}/engine/list`, () => {
          return new HttpResponse(null, { status: 500, statusText: 'Internal Server Error' });
        })
      );

      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByText('Select Engine')).toBeInTheDocument();
      });

      const trigger = screen.getByText('Select Engine');
      fireEvent.click(trigger);

      // When API fails, data is undefined so no items are rendered in the dropdown
      // Note: The component currently doesn't display an error message for API failures
      await waitFor(() => {
        // The dropdown should be open but have no engine items
        expect(screen.queryByText('engine-1')).not.toBeInTheDocument();
        expect(screen.queryByText('engine-2')).not.toBeInTheDocument();
        expect(screen.queryByText('engine-3')).not.toBeInTheDocument();
      });
    });
  });

  describe('Accessibility', () => {
    it('has proper heading hierarchy', async () => {
      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: /query profiler/i })).toBeInTheDocument();
      });

      const heading = screen.getByRole('heading', { name: /query profiler/i });
      expect(heading.tagName).toBe('H1');
    });

    it('has labels for each select input', async () => {
      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByText('Engine')).toBeInTheDocument();
      });

      // Check that labels exist
      expect(screen.getByText('Engine')).toBeInTheDocument();
      expect(screen.getByText('Coordinator')).toBeInTheDocument();
      expect(screen.getByText('Query')).toBeInTheDocument();

      // Check labels have proper htmlFor attributes
      const engineLabel = screen.getByText('Engine');
      expect(engineLabel.tagName).toBe('LABEL');
      expect(engineLabel).toHaveAttribute('for', 'engineId');

      const coordinatorLabel = screen.getByText('Coordinator');
      expect(coordinatorLabel.tagName).toBe('LABEL');
      expect(coordinatorLabel).toHaveAttribute('for', 'coordinatorId');

      const queryLabel = screen.getByText('Query');
      expect(queryLabel.tagName).toBe('LABEL');
      expect(queryLabel).toHaveAttribute('for', 'queryId');
    });

    it('has combobox role on select triggers', async () => {
      renderWithRouter({ initialPath: '/profile' });

      await waitFor(() => {
        expect(screen.getByText('Select Engine')).toBeInTheDocument();
      });

      // Get all comboboxes
      const comboboxes = screen.getAllByRole('combobox');
      expect(comboboxes.length).toBe(3);
    });
  });
});
